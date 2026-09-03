use std::{path::Path, sync::Arc};

use anyhow::{Context as _, Result, anyhow, bail};
use ghost_shell_dbus::IconPixmap;
use gpui::RenderImage;
use image::{Frame, ImageReader, RgbaImage};

pub(crate) struct TrayIcon(Arc<RenderImage>);

impl TrayIcon {
    pub(crate) fn into_inner(&self) -> Arc<RenderImage> {
        self.0.clone()
    }

    pub(crate) fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let mut image = ImageReader::open(path)
            .with_context(|| format!("failed to open tray icon {}", path.display()))?
            .with_guessed_format()
            .with_context(|| {
                format!("failed to detect tray icon format {}", path.display())
            })?
            .decode()
            .with_context(|| format!("failed to decode tray icon {}", path.display()))?
            .into_rgba8();

        // image gives us RGBA, while GPUI RenderImage expects BGRA.
        for pixel in image.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }

        Ok(Self(Arc::new(RenderImage::new([Frame::new(image)]))))
    }

    pub(crate) fn from_pixmap(pixmap: &IconPixmap) -> Result<Self> {
        let width =
            u32::try_from(pixmap.width).context("tray icon pixmap has invalid width")?;

        let height = u32::try_from(pixmap.height)
            .context("tray icon pixmap has invalid height")?;

        let expected_len = usize::try_from(width)
            .context("tray icon pixmap width is too large")?
            .checked_mul(
                usize::try_from(height)
                    .context("tray icon pixmap height is too large")?,
            )
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| anyhow!("tray icon pixmap dimensions overflow"))?;

        if pixmap.argb.len() != expected_len {
            bail!(
                "invalid tray icon pixmap buffer length: expected {expected_len}, got {}",
                pixmap.argb.len()
            );
        }

        let mut bgra = pixmap.argb.clone();

        // StatusNotifier IconPixmap gives us ARGB, while GPUI expects BGRA.
        for pixel in bgra.chunks_exact_mut(4) {
            pixel.reverse();
        }

        let image = RgbaImage::from_raw(width, height, bgra)
            .ok_or_else(|| anyhow!("failed to create tray icon image buffer"))?;

        Ok(Self(Arc::new(RenderImage::new([Frame::new(image)]))))
    }
}
