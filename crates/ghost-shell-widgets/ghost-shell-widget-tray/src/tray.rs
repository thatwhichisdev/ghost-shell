use std::{
    cmp::Reverse,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use ghost_shell_dbus::{
    DbusIntegration, IconPixmap, StatusNotifierEvent, StatusNotifierId,
    StatusNotifierItem,
};
use gpui::{
    AnyElement, Context, ObjectFit, RenderImage, Subscription, Window, div, img,
    prelude::*, px,
};

const ICON_SIZE: f32 = 20.0;
const ITEM_SIZE: f32 = 24.0;

struct TrayItem {
    item: StatusNotifierItem,
    icon: Option<TrayIcon>,
}

#[derive(Clone)]
enum TrayIcon {
    Path(PathBuf),
    Pixmap(Arc<RenderImage>),
}

impl TrayItem {
    fn new(item: StatusNotifierItem, theme: Option<&str>) -> Self {
        let icon = resolve_icon(&item, theme);

        Self { item, icon }
    }
}

impl TrayIcon {
    fn render(&self) -> AnyElement {
        match self {
            Self::Path(path) => img(path.clone()),
            Self::Pixmap(image) => img(image.clone()),
        }
        .size(px(ICON_SIZE))
        .object_fit(ObjectFit::Contain)
        .flex_none()
        .into_any_element()
    }
}

pub struct TrayWidget {
    items: HashMap<StatusNotifierId, TrayItem>,

    #[allow(unused)]
    subscription: Subscription,
}

impl TrayWidget {
    #[must_use]
    pub fn new(cx: &mut Context<Self>) -> Self {
        let icon_theme = freedesktop_icons::default_theme_gtk();
        let status_notifier = cx
            .global::<DbusIntegration>()
            .status_notifier()
            .clone();

        let items = status_notifier
            .read(cx)
            .items()
            .map(|item| {
                (
                    item.id.clone(),
                    TrayItem::new(item.clone(), icon_theme.as_deref()),
                )
            })
            .collect();

        let event_icon_theme = icon_theme.clone();
        let subscription = cx.subscribe(&status_notifier, move |widget, _, event, cx| {
            match event {
                StatusNotifierEvent::Added(item) | StatusNotifierEvent::Updated(item) => {
                    log::info!("added {item:?}");
                    widget.items.insert(
                        item.id.clone(),
                        TrayItem::new(item.clone(), event_icon_theme.as_deref()),
                    );
                }
                StatusNotifierEvent::Removed(id) => {
                    log::info!("removed {id:#}");
                    widget.items.remove(id);
                }
            }

            cx.notify();
        });

        Self {
            items,
            subscription,
        }
    }

    pub fn items(&self) -> impl Iterator<Item = &StatusNotifierItem> {
        self.items.values().map(|item| &item.item)
    }

    fn render_item(item: &TrayItem) -> Option<AnyElement> {
        let icon = item.icon.as_ref()?;

        Some(
            div()
                .id(item.item.id.to_string())
                .size(px(ITEM_SIZE))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(icon.render())
                .into_any_element(),
        )
    }
}

impl Render for TrayWidget {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut items: Vec<&TrayItem> = self.items.values().collect();
        items.sort_by(|left, right| {
            left.item
                .id
                .service()
                .cmp(right.item.id.service())
                .then_with(|| {
                    left.item
                        .id
                        .object_path()
                        .cmp(right.item.id.object_path())
                })
        });

        div()
            .id("tray")
            .flex()
            .items_center()
            .gap_1()
            .children(
                items
                    .into_iter()
                    .filter_map(Self::render_item),
            )
    }
}

fn resolve_icon(item: &StatusNotifierItem, theme: Option<&str>) -> Option<TrayIcon> {
    let needs_attention = item
        .status
        .as_deref()
        .is_some_and(|status| status.eq_ignore_ascii_case("NeedsAttention"));

    if needs_attention
        && let Some(icon) = resolve_icon_source(
            item.attention_icon_name.as_deref(),
            &item.attention_icon_pixmaps,
            theme,
            item.icon_theme_path.as_deref(),
        )
    {
        return Some(icon);
    }

    resolve_icon_source(
        item.icon_name.as_deref(),
        &item.icon_pixmaps,
        theme,
        item.icon_theme_path.as_deref(),
    )
}

fn resolve_icon_source(
    name: Option<&str>,
    pixmaps: &[IconPixmap],
    theme: Option<&str>,
    theme_path: Option<&str>,
) -> Option<TrayIcon> {
    name.and_then(|name| resolve_icon_path(name, theme, theme_path))
        .map(TrayIcon::Path)
        .or_else(|| render_pixmap(pixmaps).map(TrayIcon::Pixmap))
}

fn resolve_icon_path(
    name: &str,
    theme: Option<&str>,
    theme_path: Option<&str>,
) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }

    let path = Path::new(name);
    if path.is_absolute() && path.is_file() {
        return Some(path.to_owned());
    }

    if let Some(theme_path) = theme_path {
        let theme_path = Path::new(theme_path);
        let direct_path = theme_path.join(name);
        if direct_path.is_file() {
            return Some(direct_path);
        }

        for extension in ["png", "svg", "xpm"] {
            let path = theme_path.join(format!("{name}.{extension}"));
            if path.is_file() {
                return Some(path);
            }
        }
    }

    let mut lookup = freedesktop_icons::lookup(name)
        .with_size(ICON_SIZE as u16)
        .with_scale(1)
        .with_cache();
    if let Some(theme) = theme {
        lookup = lookup.with_theme(theme);
    }

    lookup.find()
}

fn render_pixmap(pixmaps: &[IconPixmap]) -> Option<Arc<RenderImage>> {
    let target_size = ICON_SIZE as i32;
    let pixmap = pixmaps
        .iter()
        .filter(|pixmap| valid_pixmap(pixmap))
        .min_by_key(|pixmap| {
            let edge = pixmap.width.max(pixmap.height);
            let area = i64::from(pixmap.width) * i64::from(pixmap.height);
            ((edge - target_size).unsigned_abs(), Reverse(area))
        })?;

    let width = u32::try_from(pixmap.width).ok()?;
    let height = u32::try_from(pixmap.height).ok()?;
    let byte_count = usize::try_from(width)
        .ok()?
        .checked_mul(usize::try_from(height).ok()?)?
        .checked_mul(4)?;
    let mut bgra = Vec::with_capacity(byte_count);

    for pixel in pixmap
        .argb
        .get(..byte_count)?
        .chunks_exact(4)
    {
        bgra.extend_from_slice(&[pixel[3], pixel[2], pixel[1], pixel[0]]);
    }

    let buffer = image::RgbaImage::from_raw(width, height, bgra)?;
    Some(Arc::new(RenderImage::new([image::Frame::new(buffer)])))
}

fn valid_pixmap(pixmap: &IconPixmap) -> bool {
    let Ok(width) = usize::try_from(pixmap.width) else {
        return false;
    };
    let Ok(height) = usize::try_from(pixmap.height) else {
        return false;
    };

    width > 0
        && height > 0
        && width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .is_some_and(|byte_count| pixmap.argb.len() >= byte_count)
}

#[cfg(test)]
mod tests {
    use ghost_shell_dbus::IconPixmap;

    use super::render_pixmap;

    #[test]
    fn converts_status_notifier_argb_to_gpui_bgra() {
        let pixmap = IconPixmap {
            width: 1,
            height: 1,
            argb: vec![0x80, 0x10, 0x20, 0x30],
        };

        let image = render_pixmap(&[pixmap]).expect("valid pixmap");

        assert_eq!(image.as_bytes(0), Some([0x30, 0x20, 0x10, 0x80].as_slice()));
    }

    #[test]
    fn rejects_pixmaps_with_incomplete_pixel_data() {
        let pixmap = IconPixmap {
            width: 2,
            height: 2,
            argb: vec![0; 4],
        };

        assert!(render_pixmap(&[pixmap]).is_none());
    }
}
