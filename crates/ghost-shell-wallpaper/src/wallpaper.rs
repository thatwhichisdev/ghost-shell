use std::{
    ffi::OsStr,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result};
use gpui::{
    App, AppContext as _, Context, Entity, Global, IntoElement, ObjectFit, Render,
    RenderImage, Rgba, Styled as _, StyledImage as _, Subscription, Task, Window, div,
    img, rgba,
};
use image::{AnimationDecoder, ImageDecoder as _, ImageReader, codecs::gif::GifDecoder};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use ghost_shell_config::AppConfig;

use crate::cache;

pub struct WallpaperManager {
    pub source: WallpaperSource,
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
    pub patch: Box<[u8]>,
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

        Self { source }
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
            .collect::<Vec<_>>();

        // calculate additional delta to connect last frame with initial
        if let (Some(first), Some(last)) = (frames.first(), frames.last()) {
            deltas.push(FrameDelta::between(last, first));
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
}

impl AnimatedWallpaper {
    pub fn new(source: Arc<Animation>) -> Self {
        let frame = source.initial.pixels.to_vec();
        let image = Self::render(source.width, source.height, &frame);
        let delta_buffer = vec![0; frame.len()];

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

    fn advance(&mut self, window: &mut Window) {
        if self.source.deltas.is_empty() {
            return;
        }

        let delta = &self.source.deltas[self.frame_index];
        delta.apply(&mut self.frame, &mut self.delta_buffer);

        let next = Self::render(self.source.width, self.source.height, &self.frame);
        let prev = std::mem::replace(&mut self.image, next);

        self.frame_index = (self.frame_index + 1) % self.source.deltas.len();

        window.drop_image(prev).unwrap();
    }
}

impl WallpaperSource {
    pub fn entity(&self, window: &mut Window, cx: &mut App) -> Entity<Wallpaper> {
        match &self {
            Self::Animated(animation) => {
                let animation = animation.clone();
                cx.new(|cx| Wallpaper::animated(animation, window, cx))
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
                        wallpaper.animation_mut().advance(window);
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
    fn between(first: &DecodedFrame, second: &DecodedFrame) -> FrameDelta {
        let mut delta: Vec<u8> = vec![0u8; first.pixels.len()];

        for ((out, a), b) in delta
            .iter_mut()
            .zip(first.pixels.iter())
            .zip(second.pixels.iter())
        {
            *out = a ^ b;
        }

        let patch = lz4_flex::block::compress(&delta);

        FrameDelta {
            patch: patch.into_boxed_slice(),
            delay: second.delay,
        }
    }

    pub fn apply(&self, frame: &mut [u8], delta_buffer: &mut [u8]) {
        let len = lz4_flex::block::decompress_into(&self.patch, delta_buffer).unwrap();

        for (pixel, delta) in frame.iter_mut().zip(&delta_buffer[..len]) {
            *pixel ^= *delta;
        }
    }
}
