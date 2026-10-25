use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, OnceLock, Weak,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use iced::{
    Rectangle, mouse, wgpu,
    widget::shader::{self, Viewport},
};

use crate::Animation;

pub struct Wallpaper(pub Arc<Animation>);
#[derive(Default)]
pub struct State {
    playback: OnceLock<Arc<Mutex<Playback>>>,
}
#[derive(Debug)]
struct Playback {
    id: u64,
    source: Arc<Animation>,
    pixels: Vec<u8>,
    scratch: Vec<u8>,
    index: usize,
    generation: u64,
    deadline: Instant,
    failed: bool,
}
impl Playback {
    fn new(source: Arc<Animation>) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            pixels: source.initial.pixels.to_vec(),
            scratch: vec![
                0;
                source
                    .deltas
                    .iter()
                    .map(|delta| delta.decoded_len)
                    .max()
                    .unwrap_or(0)
            ],
            index: 0,
            generation: 0,
            deadline: Instant::now() + source.initial.delay,
            source,
            failed: false,
        }
    }
    fn advance(&mut self, now: Instant) {
        // After occlusion, resume rather than reconstructing minutes of missed frames.
        let mut advanced = 0;
        while self.deadline <= now && !self.failed && advanced < 4 {
            let Some(delta) = self.source.deltas.get(self.index) else {
                break;
            };
            if let Err(error) = delta.apply(&mut self.pixels, &mut self.scratch) {
                log::error!("Wallpaper playback failed: {error:#}");
                self.failed = true;
                break;
            }
            self.deadline += delta.delay.max(Duration::from_millis(10));
            self.index = (self.index + 1) % self.source.deltas.len();
            self.generation = self.generation.wrapping_add(1);
            advanced += 1;
        }
        if self.deadline <= now {
            self.deadline = now + Duration::from_millis(10);
        }
    }
}
impl State {
    fn playback(&self, source: &Arc<Animation>) -> Arc<Mutex<Playback>> {
        self.playback
            .get_or_init(|| Arc::new(Mutex::new(Playback::new(source.clone()))))
            .clone()
    }
}
impl<Message> shader::Program<Message> for Wallpaper {
    type State = State;
    type Primitive = Primitive;
    fn update(
        &self,
        state: &mut State,
        event: &iced::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<shader::Action<Message>> {
        if !matches!(
            event,
            iced::Event::Window(iced::window::Event::RedrawRequested(_))
        ) || self.0.deltas.is_empty()
        {
            return None;
        }
        let playback = state.playback(&self.0);
        let mut playback = match playback.lock() {
            Ok(playback) => playback,
            Err(error) => {
                log::error!("Wallpaper state poisoned: {error}");
                return None;
            }
        };
        playback.advance(Instant::now());
        (!playback.failed).then(|| shader::Action::request_redraw_at(playback.deadline))
    }
    fn draw(
        &self,
        state: &State,
        _cursor: mouse::Cursor,
        bounds: Rectangle,
    ) -> Primitive {
        Primitive {
            playback: state.playback(&self.0),
            bounds,
        }
    }
}
#[derive(Debug)]
pub struct Primitive {
    playback: Arc<Mutex<Playback>>,
    bounds: Rectangle,
}
struct Texture {
    owner: Weak<Mutex<Playback>>,
    texture: wgpu::Texture,
    uniforms: wgpu::Buffer,
    group: wgpu::BindGroup,
    generation: Option<u64>,
    viewport: Rectangle,
}
pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    textures: HashMap<u64, Texture>,
}
impl shader::Pipeline for Pipeline {
    fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ghost wallpaper"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ghost wallpaper"),
            source: wgpu::ShaderSource::Wgsl(include_str!("wallpaper.wgsl").into()),
        });
        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&layout],
                push_constant_ranges: &[],
            });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ghost wallpaper"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            pipeline,
            layout,
            sampler,
            textures: HashMap::new(),
        }
    }
    fn trim(&mut self) {
        self.textures
            .retain(|_, texture| texture.owner.strong_count() > 0);
    }
}
impl shader::Primitive for Primitive {
    type Pipeline = Pipeline;
    fn prepare(
        &self,
        pipeline: &mut Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        let playback = match self.playback.lock() {
            Ok(playback) => playback,
            Err(error) => {
                log::error!("Wallpaper upload: {error}");
                return;
            }
        };
        let source = &playback.source;
        if source.width > device.limits().max_texture_dimension_2d
            || source.height > device.limits().max_texture_dimension_2d
        {
            return;
        }
        let size = wgpu::Extent3d {
            width: source.width,
            height: source.height,
            depth_or_array_layers: 1,
        };
        let texture = pipeline
            .textures
            .entry(playback.id)
            .or_insert_with(|| {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("ghost wallpaper"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                let view = texture.create_view(&Default::default());
                let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: 16,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &pipeline.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: uniforms.as_entire_binding(),
                        },
                    ],
                });
                Texture {
                    owner: Arc::downgrade(&self.playback),
                    texture,
                    uniforms,
                    group,
                    generation: None,
                    viewport: Rectangle::default(),
                }
            });
        if texture.generation != Some(playback.generation) {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &playback.pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(source.width * 4),
                    rows_per_image: Some(source.height),
                },
                size,
            );
            texture.generation = Some(playback.generation);
        }
        let scale = viewport.scale_factor() as f32;
        texture.viewport = Rectangle {
            x: self.bounds.x * scale,
            y: self.bounds.y * scale,
            width: self.bounds.width * scale,
            height: self.bounds.height * scale,
        };
        let target_ratio = self.bounds.width / self.bounds.height.max(1.0);
        let image_ratio = source.width as f32 / source.height as f32;
        let crop = if image_ratio > target_ratio {
            [target_ratio / image_ratio, 1.0, 0.0, 0.0]
        } else {
            [1.0, image_ratio / target_ratio, 0.0, 0.0]
        };
        let mut bytes = [0; 16];
        for (destination, value) in bytes.chunks_exact_mut(4).zip(crop) {
            destination.copy_from_slice(&value.to_ne_bytes());
        }
        queue.write_buffer(&texture.uniforms, 0, &bytes);
    }
    fn render(
        &self,
        pipeline: &Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip: &Rectangle<u32>,
    ) {
        let id = match self.playback.lock() {
            Ok(playback) => playback.id,
            Err(_) => return,
        };
        let Some(texture) = pipeline.textures.get(&id) else {
            return;
        };
        if clip.width == 0
            || clip.height == 0
            || texture.viewport.width <= 0.0
            || texture.viewport.height <= 0.0
        {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ghost wallpaper"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let bounds = texture.viewport;
        pass.set_viewport(bounds.x, bounds.y, bounds.width, bounds.height, 0.0, 1.0);
        pass.set_scissor_rect(clip.x, clip.y, clip.width, clip.height);
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &texture.group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DecodedFrame, Frame, FrameDelta};
    #[test]
    fn late_frames_keep_delta_chain_and_bound_catchup() -> anyhow::Result<()> {
        let first = DecodedFrame {
            pixels: vec![255, 0, 0, 255],
            delay: Duration::from_millis(20),
        };
        let second = DecodedFrame {
            pixels: vec![0, 0, 255, 255],
            delay: Duration::from_millis(20),
        };
        let source = Arc::new(Animation {
            width: 1,
            height: 1,
            initial: Frame {
                pixels: first.pixels.clone().into_boxed_slice(),
                delay: first.delay,
            },
            deltas: vec![
                FrameDelta::between(&first, &second)?,
                FrameDelta::between(&second, &first)?,
            ],
        });
        let mut playback = Playback::new(source);
        let now = playback.deadline;
        playback.advance(now);
        assert_eq!(playback.pixels, second.pixels);
        assert_eq!(playback.generation, 1);
        playback.advance(now + Duration::from_secs(60));
        assert_eq!(playback.generation, 5);
        assert_eq!(playback.pixels, second.pixels);
        assert!(playback.deadline > now + Duration::from_secs(60));
        Ok(())
    }
    #[test]
    #[ignore = "requires a graphics adapter; run explicitly on a GPU host"]
    fn persistent_texture_shader_passes_gpu_validation() -> anyhow::Result<()> {
        use shader::{Pipeline as _, Primitive as _};
        let runtime = tokio::runtime::Builder::new_current_thread().build()?;
        runtime.block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await?;
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await?;
            device.push_error_scope(wgpu::ErrorFilter::Validation);
            let mut pipeline =
                Pipeline::new(&device, &queue, wgpu::TextureFormat::Rgba8UnormSrgb);
            let source = Arc::new(Animation {
                width: 1,
                height: 1,
                initial: Frame {
                    pixels: vec![255, 0, 0, 255].into_boxed_slice(),
                    delay: Duration::ZERO,
                },
                deltas: vec![],
            });
            let playback = Arc::new(Mutex::new(Playback::new(source)));
            let primitive = Primitive {
                playback,
                bounds: Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: 16.0,
                    height: 16.0,
                },
            };
            let viewport = Viewport::with_physical_size(iced::Size::new(16, 16), 1.0);
            primitive.prepare(
                &mut pipeline,
                &device,
                &queue,
                &primitive.bounds,
                &viewport,
            );
            primitive.prepare(
                &mut pipeline,
                &device,
                &queue,
                &primitive.bounds,
                &viewport,
            );
            assert_eq!(pipeline.textures.len(), 1);
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: 16,
                    height: 16,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let mut encoder = device.create_command_encoder(&Default::default());
            primitive.render(
                &pipeline,
                &mut encoder,
                &target.create_view(&Default::default()),
                &Rectangle {
                    x: 0,
                    y: 0,
                    width: 16,
                    height: 16,
                },
            );
            queue.submit([encoder.finish()]);
            if let Some(error) = device.pop_error_scope().await {
                anyhow::bail!("{error}");
            }
            drop(primitive);
            pipeline.trim();
            assert!(pipeline.textures.is_empty());
            Ok(())
        })
    }
}
