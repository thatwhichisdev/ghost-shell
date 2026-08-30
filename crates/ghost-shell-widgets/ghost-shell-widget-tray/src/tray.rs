use anyhow::{Context as _, Result, anyhow, bail};
use std::{collections::BTreeMap, path::Path, sync::Arc};

use ghost_shell_dbus::{
    Dbus, IconPixmap, StatusNotifierEvent, StatusNotifierId, StatusNotifierItem,
};
use gpui::{
    Context, ObjectFit, RenderImage, Subscription, Window, div, img, prelude::*, px,
};
use image::{Frame, ImageReader, RgbaImage};

const ICON_SIZE: f32 = 18.0;
const ITEM_SIZE: f32 = 18.0;

struct TrayIcon(Arc<RenderImage>);

impl TrayIcon {
    fn into_inner(&self) -> Arc<RenderImage> {
        self.0.clone()
    }
}

struct TrayItem {
    icon: TrayIcon,
}

pub struct TrayWidget {
    items: BTreeMap<StatusNotifierId, TrayItem>,
    _subscription: Subscription,
}

impl TrayWidget {
    #[must_use]
    pub fn new(cx: &mut Context<Self>) -> Self {
        let status_notifier = cx.global::<Dbus>().status_notifier().clone();

        let items = status_notifier
            .read(cx)
            .items()
            .filter_map(|item| {
                TrayItem::try_from(item)
                    .ok()
                    .map(|tray_item| (item.id.clone(), tray_item))
            })
            .collect();

        let subscription = cx.subscribe(&status_notifier, |tray, _state, event, cx| {
            match event {
                StatusNotifierEvent::Registered(item)
                | StatusNotifierEvent::Updated(item) => {
                    let id = item.id.clone();
                    match TrayItem::try_from(item) {
                        Ok(item) => {
                            tray.items.insert(id, item);
                        }
                        Err(err) => {
                            log::error!("failed to create tray item {err:#}");
                        }
                    }
                }
                StatusNotifierEvent::Unregistered(id) => {
                    tray.items.remove(id);
                }
            }

            cx.notify();
        });

        Self {
            items,
            _subscription: subscription,
        }
    }
}

impl Render for TrayWidget {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let items: Vec<_> = self.items.iter().collect();

        div()
            .id("tray")
            .flex()
            .items_center()
            .gap_1()
            .children(items.into_iter().map(|(id, item)| {
                div()
                    .id(id.to_string())
                    .size(px(ITEM_SIZE))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        img(item.icon.into_inner().clone())
                            .size(px(ICON_SIZE))
                            .object_fit(ObjectFit::Contain),
                    )
            }))
    }
}

impl TryFrom<&StatusNotifierItem> for TrayItem {
    type Error = anyhow::Error;

    fn try_from(value: &StatusNotifierItem) -> Result<Self> {
        let icon = if value.icon_pixmaps.is_empty() {
            let path = value
                .icon_name
                .clone()
                .ok_or_else(|| anyhow!("status notifier item has no icon"))?;

            TrayIcon::from_path(path)?
        } else {
            let pixmap = value
                .icon_pixmaps
                .iter()
                .filter(|pixmap| pixmap.width > 0 && pixmap.height > 0)
                .max_by_key(|pixmap| i64::from(pixmap.width) * i64::from(pixmap.height))
                .ok_or_else(|| {
                    anyhow!("status notifier item has no usable icon pixmap")
                })?;

            TrayIcon::from_pixmap(pixmap)?
        };

        Ok(Self { icon })
    }
}

impl TrayIcon {
    fn from_path(path: impl AsRef<Path>) -> Result<Self> {
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

    fn from_pixmap(pixmap: &IconPixmap) -> Result<Self> {
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
