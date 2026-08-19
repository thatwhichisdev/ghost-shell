use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context as _, Result};
use ghost_shell_config::AppConfig;
use gpui::{
    App, Context, Global, IntoElement, ObjectFit, ParentElement as _, Render,
    RenderImage, Styled as _, StyledImage as _, Task, Window, div, img,
};
use image::{AnimationDecoder, ImageDecoder as _, codecs::gif::GifDecoder};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::cache;

pub struct WallpaperManager {
    source: Option<WallpaperSource>,
}

#[derive(Clone)]
pub enum WallpaperSource {
    Animated(Arc<AnimatedWallpaper>),
    Static(Arc<StaticWallpaper>),
}

/// Immutable decoded animated wallpaper.
///
/// Shared between every Wallpaper entity.
#[derive(Serialize, Deserialize)]
pub struct AnimatedWallpaper {
    pub width: u32,
    pub height: u32,

    pub loop_count: u32,

    pub initial: Frame,
    pub deltas: Vec<FrameDelta>,
}

/// One independent playback/rendering instance.
///
/// This becomes Entity<Wallpaper>.
pub struct Wallpaper {
    source: Arc<AnimatedWallpaper>,

    /// Current reconstructed BGRA framebuffer.
    frame: Vec<u8>,

    /// Reused decompression buffer.
    scratch: Vec<u8>,

    /// Currently rendered GPUI image.
    image: Arc<RenderImage>,

    /// Index of the next delta to apply.
    frame_index: usize,

    /// Relevant later for finite GIFs.
    completed_loops: u32,

    _task: Task<()>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Frame {
    /// Full BGRA8 framebuffer.
    pub pixels: Box<[u8]>,
    pub delay: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FrameDelta {
    /// LZ4-compressed XOR against the previous frame.
    pub patch: Box<[u8]>,
    pub delay: Duration,
}

#[derive(Clone)]
struct DecodedFrame {
    pixels: Vec<u8>,
    delay: Duration,
}

pub struct StaticWallpaper;

impl Global for WallpaperManager {}

impl WallpaperManager {
    pub fn new(cx: &mut App) -> Self {
        let source = cx
            .global::<AppConfig>()
            .wallpaper
            .path
            .clone()
            .map(|path| WallpaperSource::load(PathBuf::from(path)).unwrap());

        Self { source }
    }

    pub fn source(&self) -> Option<WallpaperSource> {
        self.source.clone()
    }
}

impl Wallpaper {
    pub fn new(
        source: Arc<AnimatedWallpaper>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Self> {
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
            _task: Self::spawn_task(window, cx),
        })
    }

    fn spawn_task(window: &mut Window, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn_in(window, async move |wallpaper, cx| {
            loop {
                let delay = match wallpaper.update(cx, |wallpaper, _| wallpaper.delay()) {
                    Ok(delay) => delay,
                    Err(_) => break,
                };

                cx.background_executor().timer(delay).await;

                let result = wallpaper.update_in(cx, |wallpaper, window, cx| {
                    if !window.is_window_active() {
                        return Ok(true);
                    }

                    wallpaper.advance(window, cx)
                });

                match result {
                    Ok(Ok(true)) => {}
                    Ok(Ok(false)) => break,
                    Ok(Err(error)) => {
                        log::error!("failed to advance wallpaper: {error:#}");
                        break;
                    }
                    Err(_) => break,
                }
            }
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

    fn advance(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<bool> {
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

            self.frame.copy_from_slice(&self.source.initial.pixels);

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
    fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file_name = path
            .file_name()
            .context("wallpaper path has no file name")?;

        let file = File::open(path)
            .with_context(|| format!("failed to find wallpaper {}", path.display()))?;

        if let Some(wallpaper) = cache::load(file_name)? {
            return Ok(Self::Animated(Arc::new(wallpaper)));
        }

        let decoder = GifDecoder::new(BufReader::new(file))
            .with_context(|| format!("failed to decode wallpaper {}", path.display()))?;

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

        let animated = AnimatedWallpaper {
            width,
            height,
            loop_count,
            initial,
            deltas,
        };

        cache::save(file_name, &animated)?;

        Ok(Self::Animated(Arc::new(animated)))
    }
}

impl Render for Wallpaper {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().relative().size_full().child(
            img(self.image.clone())
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .object_fit(ObjectFit::Cover),
        )
    }
}

impl Frame {
    fn diff(first: &DecodedFrame, second: &DecodedFrame) -> FrameDelta {
        let start = std::time::Instant::now();

        let mut delta: Vec<u8> = vec![0u8; first.pixels.len()];

        for ((delta, first), second) in
            delta.iter_mut().zip(&first.pixels).zip(&second.pixels)
        {
            *delta = first ^ second;
        }

        let patch = lz4_flex::block::compress(&delta);

        let finish = std::time::Instant::now().duration_since(start).as_secs();
        log::info!("Calculated frames delta in {finish}s");

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
