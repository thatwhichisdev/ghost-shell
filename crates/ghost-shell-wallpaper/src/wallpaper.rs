use std::{
    ffi::OsStr,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result};
use ghost_shell_app::GhostShell;
use ghost_shell_config::AppConfig;
use gpui::{
    App, AppContext as _, Bounds, Context, DevicePixels, Entity, Global, IntoElement,
    ObjectFit, Point, Render, RenderImage, Rgba, Size, Styled as _, StyledImage as _,
    Subscription, Task, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle,
    WindowKind, WindowOptions, div, img,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    px, rgba,
};
use image::{AnimationDecoder, ImageDecoder as _, ImageReader, codecs::gif::GifDecoder};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

mod cache;

pub fn init(cx: &mut App) {
    let mut manager = WallpaperManager::new(cx);
    manager.open(cx);

    cx.set_global(manager);
}

pub struct WallpaperManager {
    pub source: WallpaperSource,
    windows: Vec<WindowHandle<Wallpaper>>,
}

#[derive(Clone)]
pub enum WallpaperSource {
    Animated(Arc<Animation>),
    Static(Arc<RenderImage>),
    Solid(Rgba),
}

pub enum Wallpaper {
    Animated(AnimatedWallpaper),
    Static(Arc<RenderImage>),
    Solid(Rgba),
}

pub struct AnimatedWallpaper {
    source: Arc<Animation>,

    image: Arc<RenderImage>,

    delta_buffer: Vec<u8>,

    frame: Vec<u8>,
    frame_index: usize,

    _task: Option<Task<()>>,
    _activation_subscription: Option<Subscription>,
}

#[derive(Serialize, Deserialize)]
pub struct Animation {
    pub width: u32,
    pub height: u32,
    pub initial: Frame,
    pub deltas: Vec<FrameDelta>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Frame {
    pub pixels: Box<[u8]>,
    pub delay: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FrameDelta {
    /// LZ4-compressed sparse delta stream.
    pub patch: Box<[u8]>,

    /// Size of the sparse stream after LZ4 decompression.
    pub decoded_len: usize,

    pub delay: Duration,
}

#[derive(Clone)]
struct DecodedFrame {
    pixels: Vec<u8>,
    delay: Duration,
}

impl Global for WallpaperManager {}

impl WallpaperManager {
    pub fn new(cx: &mut App) -> Self {
        let config = cx.global::<AppConfig>().wallpaper.clone();

        let source = match config.path {
            Some(path) => Self::load(PathBuf::from(path)).unwrap(),
            None => WallpaperSource::Solid(rgba(config.bg)),
        };

        Self {
            source,
            windows: Vec::new(),
        }
    }

    fn load(path: impl AsRef<Path>) -> Result<WallpaperSource> {
        let path = path.as_ref();

        let file = File::open(path)
            .with_context(|| format!("failed to open wallpaper {}", path.display()))?;

        let file_name = path
            .file_name()
            .context("wallpaper path has no file name")?;

        let reader = ImageReader::new(BufReader::new(file))
            .with_guessed_format()
            .context("failed to guess wallpaper format")?;

        let format = reader
            .format()
            .context("unsupported or unknown wallpaper format")?;

        match format {
            image::ImageFormat::Png | image::ImageFormat::Jpeg => {
                Self::load_image(file_name, reader)
            }
            image::ImageFormat::Gif => Self::load_animation(file_name, reader),
            _ => anyhow::bail!("Unsupported format"),
        }
    }

    fn load_image(
        file_name: &OsStr,
        reader: ImageReader<BufReader<File>>,
    ) -> Result<WallpaperSource> {
        let mut buffer = reader
            .decode()
            .with_context(|| {
                format!("failed to decode wallpaper {}", file_name.to_string_lossy())
            })?
            .into_rgba8();

        // GPUI RenderImage expects BGRA.
        for pixel in buffer.as_mut().chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }

        let frame = image::Frame::new(buffer);
        let image = Arc::new(RenderImage::new([frame]));

        Ok(WallpaperSource::Static(image))
    }

    fn load_animation(
        file_name: &OsStr,
        reader: ImageReader<BufReader<File>>,
    ) -> Result<WallpaperSource> {
        if let Some(animation) = cache::load(file_name)? {
            return Ok(WallpaperSource::Animated(animation));
        }

        let decoder = GifDecoder::new(reader.into_inner()).with_context(|| {
            format!("failed to decode wallpaper {}", file_name.to_string_lossy())
        })?;

        let (width, height) = decoder.dimensions();

        let frames = decoder
            .into_frames()
            .collect::<image::ImageResult<Vec<_>>>()?
            .into_par_iter()
            .map(Frame::decode)
            .collect::<Vec<_>>();

        let initial = frames[0].clone();
        let initial = Frame {
            pixels: initial.pixels.into_boxed_slice(),
            delay: initial.delay,
        };

        let mut deltas = frames
            .par_windows(2)
            .map(|pair| FrameDelta::between(&pair[0], &pair[1]))
            .collect::<Result<Vec<_>>>()?;

        // calculate additional delta to connect last frame with initial
        if let (Some(first), Some(last)) = (frames.first(), frames.last()) {
            deltas.push(FrameDelta::between(last, first)?);
        }

        let animated = Animation {
            width,
            height,
            initial,
            deltas,
        };

        cache::save(file_name, &animated)?;

        Ok(WallpaperSource::Animated(Arc::new(animated)))
    }

    pub fn open(&mut self, cx: &mut App) {
        if !self.windows.is_empty() {
            return;
        }

        for display in cx.global::<GhostShell>().get_displays() {
            let window_options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(display.bounds())),
                titlebar: None,
                focus: false,
                kind: WindowKind::LayerShell(LayerShellOptions {
                    namespace: "ghost-shell-wallpaper".to_owned(),
                    layer: Layer::Background,
                    anchor: Anchor::TOP | Anchor::RIGHT | Anchor::BOTTOM | Anchor::LEFT,
                    exclusive_zone: Some(px(-1.0)),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    ..Default::default()
                }),
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                display_id: Some(display.id()),
                window_background: WindowBackgroundAppearance::Transparent,
                app_id: Some("dev.thatwhichis.ghost-shell.wallpaper".to_owned()),
                ..Default::default()
            };

            let source = self.source.clone();

            match cx
                .open_window(window_options, move |window, cx| source.entity(window, cx))
            {
                Ok(handle) => self.windows.push(handle),
                Err(error) => {
                    log::error!(
                        "Failed to open wallpaper surface on display {:?}: {error:#}",
                        display.id(),
                    );
                }
            }
        }
    }
}

impl AnimatedWallpaper {
    pub fn new(source: Arc<Animation>) -> Self {
        let frame = source.initial.pixels.to_vec();
        let image = Self::render(source.width, source.height, &frame);

        let delta_buffer_len = source
            .deltas
            .iter()
            .map(|delta| delta.decoded_len)
            .max()
            .unwrap_or_default();

        let delta_buffer = vec![0; delta_buffer_len];

        Self {
            source,
            frame,
            delta_buffer,
            image,
            frame_index: 0,
            _task: None,
            _activation_subscription: None,
        }
    }

    fn delay(&self) -> Duration {
        if self.frame_index == 0 {
            self.source.initial.delay
        } else {
            self.source.deltas[self.frame_index - 1].delay
        }
    }

    fn render(width: u32, height: u32, pixels: &[u8]) -> Arc<RenderImage> {
        image::RgbaImage::from_raw(width, height, pixels.to_vec())
            .map(|buffer| image::Frame::new(buffer))
            .map(|frame| RenderImage::new([frame]))
            .map(|image| Arc::new(image))
            .unwrap()
    }

    fn advance(&mut self, window: &mut Window) -> Result<()> {
        if self.source.deltas.is_empty() {
            return Ok(());
        }

        let delta = &self.source.deltas[self.frame_index];
        delta.apply(&mut self.frame, &mut self.delta_buffer)?;

        let bounds = Bounds {
            origin: Point {
                x: DevicePixels(0),
                y: DevicePixels(0),
            },
            size: Size {
                width: DevicePixels(self.source.width as i32),
                height: DevicePixels(self.source.height as i32),
            },
        };

        if !window
            .update_image_region(&self.image, 0, bounds, &self.frame)
            .unwrap()
        {
            let next = Self::render(self.source.width, self.source.height, &self.frame);
            let _ = std::mem::replace(&mut self.image, next);
        }

        self.frame_index = (self.frame_index + 1) % self.source.deltas.len();

        Ok(())
    }
}

impl WallpaperSource {
    pub fn entity(&self, window: &mut Window, cx: &mut App) -> Entity<Wallpaper> {
        match &self {
            Self::Animated(animation) => {
                let animation = animation.clone();
                cx.new(|cx| {
                    let mut wallpaper = Wallpaper::animated(animation, window, cx);
                    wallpaper.start_animation(window, cx);
                    wallpaper
                })
            }

            Self::Static(image) => {
                let image = image.clone();
                cx.new(|_| Wallpaper::Static(image))
            }

            Self::Solid(solid) => {
                let solid = *solid;
                cx.new(|_| Wallpaper::Solid(solid))
            }
        }
    }
}

impl Wallpaper {
    fn animated(
        source: Arc<Animation>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let inner = AnimatedWallpaper::new(source);

        let mut wallpaper = Self::Animated(inner);

        let subscription =
            cx.observe_window_activation(window, |wallpaper, window, cx| {
                if window.is_window_active() {
                    wallpaper.start_animation(window, cx);
                } else {
                    wallpaper.stop_animation();
                }
            });

        wallpaper
            .animation_mut()
            ._activation_subscription = Some(subscription);

        if window.is_window_active() {
            wallpaper.start_animation(window, cx);
        }

        wallpaper
    }

    fn start_animation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.animation()._task.is_some() {
            return;
        }

        let task = Self::spawn_animation_task(window, cx);
        self.animation_mut()._task = Some(task);
    }

    fn stop_animation(&mut self) {
        self.animation_mut()._task = None;
    }

    fn animation(&self) -> &AnimatedWallpaper {
        match self {
            Self::Animated(animation) => animation,
            _ => unreachable!("animation task attached to non-animated wallpaper"),
        }
    }

    fn animation_mut(&mut self) -> &mut AnimatedWallpaper {
        match self {
            Self::Animated(animation) => animation,
            _ => unreachable!("animation task attached to non-animated wallpaper"),
        }
    }

    fn spawn_animation_task(window: &mut Window, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn_in(window, async move |wallpaper, cx| {
            let mut advance_time = Duration::ZERO;
            loop {
                let delay = match wallpaper
                    .update(cx, |wallpaper, _| wallpaper.animation().delay())
                {
                    Ok(delay) => delay.saturating_sub(advance_time),
                    Err(_) => break,
                };

                cx.background_executor().timer(delay).await;

                let started = Instant::now();

                if wallpaper
                    .update_in(cx, |wallpaper, window, cx| {
                        let _ = wallpaper.animation_mut().advance(window);
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }

                advance_time = started.elapsed();
            }
        })
    }
}

impl Render for Wallpaper {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        match self {
            Wallpaper::Animated(animation) => img(animation.image.clone())
                .size_full()
                .object_fit(ObjectFit::Cover)
                .into_any_element(),

            Wallpaper::Static(image) => img(image.clone())
                .size_full()
                .object_fit(ObjectFit::Cover)
                .into_any_element(),

            Wallpaper::Solid(color) => div()
                .size_full()
                .bg(*color)
                .into_any_element(),
        }
    }
}

impl Frame {
    fn decode(frame: image::Frame) -> DecodedFrame {
        let delay = frame.delay().into();
        let mut pixels = frame.into_buffer().into_raw();

        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }

        DecodedFrame { pixels, delay }
    }
}

impl FrameDelta {
    const BYTES_PER_PIXEL: usize = 4;
    const RUN_HEADER_SIZE: usize = 8;

    fn between(first: &DecodedFrame, second: &DecodedFrame) -> Result<Self> {
        anyhow::ensure!(
            first.pixels.len() == second.pixels.len(),
            "animation frames have different sizes"
        );

        anyhow::ensure!(
            first.pixels.len() % Self::BYTES_PER_PIXEL == 0,
            "animation frame is not BGRA-aligned"
        );

        let pixel_count = first.pixels.len() / Self::BYTES_PER_PIXEL;
        let mut stream = Vec::new();
        let mut cursor = 0;

        while cursor < pixel_count {
            let skip_start = cursor;

            while cursor < pixel_count
                && Self::pixel_eq(&first.pixels, &second.pixels, cursor)
            {
                cursor += 1;
            }

            let skip = cursor - skip_start;

            // Everything after the last changed run is unchanged, so there is
            // nothing else to encode.
            if cursor == pixel_count {
                break;
            }

            let changed_start = cursor;

            while cursor < pixel_count
                && !Self::pixel_eq(&first.pixels, &second.pixels, cursor)
            {
                cursor += 1;
            }

            let changed = cursor - changed_start;

            let skip = u32::try_from(skip).context("wallpaper delta skip exceeds u32")?;

            let changed =
                u32::try_from(changed).context("wallpaper delta run exceeds u32")?;

            stream.extend_from_slice(&skip.to_le_bytes());
            stream.extend_from_slice(&changed.to_le_bytes());

            let start = changed_start * Self::BYTES_PER_PIXEL;
            let end = cursor * Self::BYTES_PER_PIXEL;

            stream.extend_from_slice(&second.pixels[start..end]);
        }

        let decoded_len = stream.len();

        let patch = if stream.is_empty() {
            Vec::new()
        } else {
            lz4_flex::block::compress(&stream)
        };

        Ok(Self {
            patch: patch.into_boxed_slice(),
            decoded_len,
            delay: second.delay,
        })
    }

    #[inline]
    fn pixel_eq(first: &[u8], second: &[u8], index: usize) -> bool {
        let start = index * Self::BYTES_PER_PIXEL;
        let end = start + Self::BYTES_PER_PIXEL;

        first[start..end] == second[start..end]
    }

    pub fn apply(&self, frame: &mut [u8], delta_buffer: &mut [u8]) -> Result<()> {
        if self.decoded_len == 0 {
            return Ok(());
        }

        anyhow::ensure!(
            delta_buffer.len() >= self.decoded_len,
            "wallpaper delta buffer is too small"
        );

        let decoded_len = lz4_flex::block::decompress_into(&self.patch, delta_buffer)
            .context("failed to decompress wallpaper delta")?;

        anyhow::ensure!(
            decoded_len == self.decoded_len,
            "wallpaper delta decompressed to unexpected size"
        );

        let delta = &delta_buffer[..decoded_len];

        let mut source = 0;
        let mut pixel = 0usize;

        while source < delta.len() {
            let header_end = source
                .checked_add(Self::RUN_HEADER_SIZE)
                .context("wallpaper delta header overflow")?;

            let header = delta
                .get(source..header_end)
                .context("truncated wallpaper delta header")?;

            let skip =
                u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;

            let changed =
                u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;

            source = header_end;

            pixel = pixel
                .checked_add(skip)
                .context("wallpaper delta pixel offset overflow")?;

            let changed_bytes = changed
                .checked_mul(Self::BYTES_PER_PIXEL)
                .context("wallpaper delta byte count overflow")?;

            let literal_end = source
                .checked_add(changed_bytes)
                .context("wallpaper delta literal overflow")?;

            let literal = delta
                .get(source..literal_end)
                .context("truncated wallpaper delta pixels")?;

            let frame_start = pixel
                .checked_mul(Self::BYTES_PER_PIXEL)
                .context("wallpaper frame offset overflow")?;

            let frame_end = frame_start
                .checked_add(changed_bytes)
                .context("wallpaper frame range overflow")?;

            let destination = frame
                .get_mut(frame_start..frame_end)
                .context("wallpaper delta exceeds frame bounds")?;

            destination.copy_from_slice(literal);

            source = literal_end;

            pixel = pixel
                .checked_add(changed)
                .context("wallpaper delta pixel offset overflow")?;
        }

        Ok(())
    }
}
