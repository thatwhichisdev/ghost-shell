#![recursion_limit = "256"]
mod cache;
mod shader;

use std::{fs::File, io::BufReader, path::Path, sync::Arc, time::Duration};

use anyhow::{Context as _, Result};
use iced::{Color, Element, Fill};
use image::{
    AnimationDecoder as _, ImageDecoder as _, ImageReader, codecs::gif::GifDecoder,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum WallpaperSource {
    Image(Arc<Animation>),
    Solid(Color),
}

impl WallpaperSource {
    pub fn load(path: &Path) -> Result<Self> {
        match cache::load(path) {
            Ok(Some(animation)) => return Ok(Self::Image(Arc::new(animation))),
            Ok(None) => {}
            Err(error) => log::warn!("Ignoring wallpaper cache: {error:#}"),
        }
        let animation = Self::decode(path)?;
        if let Err(error) = cache::save(path, &animation) {
            log::warn!("wallpaper cache: {error:#}");
        }
        Ok(Self::Image(Arc::new(animation)))
    }

    fn decode(path: &Path) -> Result<Animation> {
        let reader =
            ImageReader::new(BufReader::new(File::open(path)?)).with_guessed_format()?;
        let animation = match reader.format() {
            Some(image::ImageFormat::Gif) => {
                let decoder = GifDecoder::new(reader.into_inner())?;
                let (width, height) = decoder.dimensions();
                validate_dimensions(width, height)?;
                let mut frames = decoder.into_frames();
                let initial = Frame::decode(
                    frames
                        .next()
                        .context("GIF contains no frames")??,
                );
                let mut previous = initial.clone();
                let mut deltas = Vec::new();
                let mut compressed_bytes = 0usize;
                for frame in frames {
                    let next = Frame::decode(frame?);
                    let delta = FrameDelta::between(&previous, &next)?;
                    compressed_bytes = compressed_bytes
                        .checked_add(delta.patch.len())
                        .context("animation size overflow")?;
                    anyhow::ensure!(
                        compressed_bytes <= 512 * 1024 * 1024 && deltas.len() < 100_000,
                        "animation exceeds memory limits"
                    );
                    deltas.push(delta);
                    previous = next;
                }
                if !deltas.is_empty() {
                    deltas.push(FrameDelta::between(&previous, &initial)?);
                }
                Animation {
                    width,
                    height,
                    initial: Frame {
                        pixels: initial.pixels.into_boxed_slice(),
                        delay: initial.delay,
                    },
                    deltas,
                }
            }
            Some(image::ImageFormat::Png | image::ImageFormat::Jpeg) => {
                let image = reader.decode()?.into_rgba8();
                let (width, height) = image.dimensions();
                validate_dimensions(width, height)?;
                Animation {
                    width,
                    height,
                    initial: Frame {
                        pixels: image.into_raw().into_boxed_slice(),
                        delay: Duration::ZERO,
                    },
                    deltas: Vec::new(),
                }
            }
            _ => anyhow::bail!("unsupported wallpaper format"),
        };
        animation.validate()?;
        Ok(animation)
    }

    pub fn view<Message: 'static>(&self) -> Element<'_, Message> {
        match self {
            Self::Image(source) => {
                iced::widget::shader(shader::Wallpaper(source.clone()))
                    .width(Fill)
                    .height(Fill)
                    .into()
            }
            Self::Solid(color) => iced::widget::container(
                iced::widget::Space::new()
                    .width(Fill)
                    .height(Fill),
            )
            .style(move |_| iced::widget::container::Style::default().background(*color))
            .into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
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
    pub decoded_len: usize,
    pub delay: Duration,
}
#[derive(Clone)]
struct DecodedFrame {
    pixels: Vec<u8>,
    delay: Duration,
}

fn validate_dimensions(width: u32, height: u32) -> Result<usize> {
    anyhow::ensure!(
        width > 0 && height > 0 && width <= 16384 && height <= 16384,
        "invalid wallpaper dimensions"
    );
    let bytes = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .context("wallpaper dimensions overflow")?;
    anyhow::ensure!(
        bytes <= 256 * 1024 * 1024,
        "wallpaper exceeds frame memory limit"
    );
    Ok(bytes)
}
impl Animation {
    fn validate(&self) -> Result<()> {
        let bytes = validate_dimensions(self.width, self.height)?;
        anyhow::ensure!(
            self.initial.pixels.len() == bytes,
            "invalid initial wallpaper frame"
        );
        anyhow::ensure!(self.deltas.len() <= 100_000, "too many animation frames");
        let mut frame = self.initial.pixels.to_vec();
        let mut buffer = vec![
            0;
            self.deltas
                .iter()
                .map(|delta| delta.decoded_len)
                .max()
                .unwrap_or(0)
                .min(bytes.saturating_mul(3))
        ];
        for delta in &self.deltas {
            delta.apply(&mut frame, &mut buffer)?;
        }
        anyhow::ensure!(
            self.deltas.is_empty() || frame.as_slice() == self.initial.pixels.as_ref(),
            "animation loop does not return to initial frame"
        );
        Ok(())
    }
}
impl Frame {
    fn decode(frame: image::Frame) -> DecodedFrame {
        DecodedFrame {
            delay: Duration::from(frame.delay()).max(Duration::from_millis(10)),
            pixels: frame.into_buffer().into_raw(),
        }
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
            "animation frame is not RGBA-aligned"
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

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(pixels: &[u8]) -> DecodedFrame {
        DecodedFrame {
            pixels: pixels.to_vec(),
            delay: Duration::from_millis(20),
        }
    }
    #[test]
    fn sparse_deltas_reconstruct_changed_runs_and_wrap() -> Result<()> {
        let first = frame(&[1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255]);
        let second = frame(&[9, 8, 7, 255, 4, 5, 6, 255, 3, 2, 1, 255]);
        let forward = FrameDelta::between(&first, &second)?;
        let backward = FrameDelta::between(&second, &first)?;
        let mut pixels = first.pixels.clone();
        let mut scratch = vec![0; forward.decoded_len.max(backward.decoded_len)];
        forward.apply(&mut pixels, &mut scratch)?;
        assert_eq!(pixels, second.pixels);
        backward.apply(&mut pixels, &mut scratch)?;
        assert_eq!(pixels, first.pixels);
        let unchanged = FrameDelta::between(&first, &first)?;
        assert!(unchanged.patch.is_empty());
        unchanged.apply(&mut pixels, &mut [])?;
        assert_eq!(pixels, first.pixels);
        Ok(())
    }
    #[test]
    fn corrupt_delta_is_rejected_without_out_of_bounds_access() {
        for stream in [
            vec![1, 2, 3],
            [0u32.to_le_bytes(), 2u32.to_le_bytes()].concat(),
            [u32::MAX.to_le_bytes(), 1u32.to_le_bytes(), [0; 4]].concat(),
        ] {
            let delta = FrameDelta {
                patch: lz4_flex::block::compress(&stream).into_boxed_slice(),
                decoded_len: stream.len(),
                delay: Duration::ZERO,
            };
            assert!(
                delta
                    .apply(&mut [0; 4], &mut vec![0; stream.len()])
                    .is_err()
            );
        }
    }
    #[test]
    fn streamed_gif_frames_preserve_rgba_and_delays() -> Result<()> {
        let path = std::env::temp_dir()
            .join(format!("ghost-gif-test-{}.gif", std::process::id()));
        let file = File::create(&path)?;
        let mut encoder = image::codecs::gif::GifEncoder::new(file);
        encoder.set_repeat(image::codecs::gif::Repeat::Infinite)?;
        for color in [[255, 0, 0, 255], [0, 0, 255, 255]] {
            encoder.encode_frame(image::Frame::from_parts(
                image::RgbaImage::from_pixel(2, 2, image::Rgba(color)),
                0,
                0,
                image::Delay::from_numer_denom_ms(40, 1),
            ))?;
        }
        drop(encoder);
        let animation = WallpaperSource::decode(&path)?;
        assert_eq!(&animation.initial.pixels[..4], &[255, 0, 0, 255]);
        assert_eq!(animation.initial.delay, Duration::from_millis(40));
        assert_eq!(animation.deltas.len(), 2);
        let mut pixels = animation.initial.pixels.to_vec();
        let mut scratch = vec![
            0;
            animation
                .deltas
                .iter()
                .map(|delta| delta.decoded_len)
                .max()
                .unwrap_or(0)
        ];
        animation.deltas[0].apply(&mut pixels, &mut scratch)?;
        assert_eq!(&pixels[..4], &[0, 0, 255, 255]);
        std::fs::remove_file(path)?;
        Ok(())
    }
}
