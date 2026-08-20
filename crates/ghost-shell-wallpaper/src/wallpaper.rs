use std::{
    ffi::OsStr,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context as _, Result};
use ghost_shell_config::AppConfig;
use gpui::{
    App, AppContext as _, Context, Entity, Global, IntoElement, ObjectFit, Render,
    RenderImage, Rgba, Styled as _, StyledImage as _, Task, Window, div, img, rgba,
};
use image::{AnimationDecoder, ImageDecoder as _, ImageReader, codecs::gif::GifDecoder};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

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
    frame: Vec<u8>,
    scratch: Vec<u8>,
    image: Arc<RenderImage>,
    frame_index: usize,
    completed_loops: u32,
    _task: Option<Task<()>>,
}

#[derive(Serialize, Deserialize)]
pub struct Animation {
    pub width: u32,
    pub height: u32,
    pub loop_count: u32,
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
        let config = cx
            .global::<AppConfig>()
            .wallpaper
            .clone();

        let source = match config.path {
            Some(path) => Self::load(PathBuf::from(path)).unwrap(),
            None => WallpaperSource::Solid(rgba(config.bg)),
        };

        Self { source }
    }

    pub fn wallpaper(&self, window: &mut Window, cx: &mut App) -> Entity<Wallpaper> {
        match &self.source {
            WallpaperSource::Animated(animation) => cx.new(|cx| {
                let inner = AnimatedWallpaper::new(animation.clone()).unwrap();
                let mut wallpaper = Wallpaper::Animated(inner);
                wallpaper.spawn_task(window, cx);
                wallpaper
            }),

            WallpaperSource::Static(image) => {
                let image = image.clone();
                cx.new(|_| Wallpaper::Static(image))
            }

            WallpaperSource::Solid(solid) => {
                let solid = *solid;
                cx.new(|_| Wallpaper::Solid(solid))
            }
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

        let loop_count = match decoder.loop_count() {
            image::metadata::LoopCount::Infinite => 0,
            image::metadata::LoopCount::Finite(count) => count.get(),
        };

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

        let deltas = frames
            .par_windows(2)
            .map(|pair| Frame::diff(&pair[0], &pair[1]))
            .collect::<Vec<_>>();

        let animated = Animation {
            width,
            height,
            loop_count,
            initial,
            deltas,
        };

        cache::save(file_name, &animated)?;

        Ok(WallpaperSource::Animated(Arc::new(animated)))
    }
}

impl AnimatedWallpaper {
    pub fn new(source: Arc<Animation>) -> Result<Self> {
        let frame = source.initial.pixels.to_vec();
        let scratch = vec![0; frame.len()];
        let image = Self::render_image(source.width, source.height, &frame)?;

        Ok(Self {
            source,
            frame,
            scratch,
            image,
            frame_index: 0,
            completed_loops: 0,
            _task: None,
        })
    }

    fn delay(&self) -> Duration {
        if self.frame_index == 0 {
            self.source.initial.delay
        } else {
            self.source.deltas[self.frame_index - 1].delay
        }
    }

    fn render_image(width: u32, height: u32, pixels: &[u8]) -> Result<Arc<RenderImage>> {
        let expected_len = width as usize * height as usize * 4;

        anyhow::ensure!(
            pixels.len() == expected_len,
            "invalid wallpaper framebuffer size: expected {expected_len}, got {}",
            pixels.len(),
        );

        let buffer = image::RgbaImage::from_raw(width, height, pixels.to_vec())
            .context("failed to create wallpaper image buffer")?;

        let frame = image::Frame::new(buffer);

        Ok(Arc::new(RenderImage::new([frame])))
    }

    fn advance(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Wallpaper>,
    ) -> Result<bool> {
        if self.source.deltas.is_empty() {
            return Ok(false);
        }

        if self.frame_index == self.source.deltas.len() {
            if self.source.loop_count != 0
                && self.completed_loops + 1 >= self.source.loop_count
            {
                return Ok(false);
            }

            self.completed_loops += 1;
            self.frame
                .copy_from_slice(&self.source.initial.pixels);
            self.frame_index = 0;
        } else {
            let delta = &self.source.deltas[self.frame_index];
            delta.apply(&mut self.frame, &mut self.scratch)?;
            self.frame_index += 1;
        }

        let next_image =
            Self::render_image(self.source.width, self.source.height, &self.frame)?;

        let previous_image = std::mem::replace(&mut self.image, next_image);

        window.drop_image(previous_image)?;

        cx.notify();

        Ok(true)
    }
}

impl WallpaperSource {
    pub fn entity(&self, window: &mut Window, cx: &mut App) -> Entity<Wallpaper> {
        match &self {
            WallpaperSource::Animated(animation) => cx.new(|cx| {
                let inner = AnimatedWallpaper::new(animation.clone()).unwrap();
                let mut wallpaper = Wallpaper::Animated(inner);
                wallpaper.spawn_task(window, cx);
                wallpaper
            }),

            WallpaperSource::Static(image) => {
                let image = image.clone();
                cx.new(|_| Wallpaper::Static(image))
            }

            WallpaperSource::Solid(solid) => {
                let solid = *solid;
                cx.new(|_| Wallpaper::Solid(solid))
            }
        }
    }
}

impl Wallpaper {
    fn spawn_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self, Wallpaper::Animated(_)) {
            return;
        }

        let task = cx.spawn_in(window, async move |wallpaper, cx| {
            loop {
                let delay = match wallpaper.update(cx, |wallpaper, _| match wallpaper {
                    Wallpaper::Animated(animated) => Some(animated.delay()),
                    _ => None,
                }) {
                    Ok(Some(delay)) => delay,
                    _ => break,
                };

                cx.background_executor()
                    .timer(delay)
                    .await;

                let should_advance = wallpaper
                    .update_in(cx, |wallpaper, window, cx| {
                        let Wallpaper::Animated(animated) = wallpaper else {
                            return false;
                        };

                        if !window.is_window_active() {
                            return true;
                        }

                        let continue_playback = animated.advance(window, cx).unwrap();

                        cx.notify();

                        continue_playback
                    })
                    .unwrap();

                if !should_advance {
                    break;
                }
            }
        });

        if let Wallpaper::Animated(animated) = self {
            animated._task = Some(task);
        }
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
    fn diff(first: &DecodedFrame, second: &DecodedFrame) -> FrameDelta {
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
    pub fn apply(&self, frame: &mut [u8], scratch: &mut [u8]) -> Result<()> {
        let len = lz4_flex::block::decompress_into(&self.patch, scratch)?;

        anyhow::ensure!(
            len == frame.len(),
            "wallpaper delta has invalid size: expected {}, got {}",
            frame.len(),
            len,
        );

        for (pixel, delta) in frame.iter_mut().zip(&scratch[..len]) {
            *pixel ^= *delta;
        }

        Ok(())
    }
}
