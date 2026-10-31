pub mod bar;

use ghost_shell_config::{ThemeConfig, ThemeMode};
use iced::{
    Color, Element, Fill, Theme,
    theme::palette::{Background, Extended, Pair, Secondary},
    widget::{Space, column, container, scrollable},
};

pub fn font(family: &str) -> iced::Font {
    use std::{
        collections::HashMap,
        sync::{LazyLock, Mutex},
    };
    static FAMILIES: LazyLock<Mutex<HashMap<String, &'static str>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    let family = match family {
        "monospace" => iced::font::Family::Monospace,
        "sans-serif" => iced::font::Family::SansSerif,
        "serif" => iced::font::Family::Serif,
        name => {
            let mut families = FAMILIES
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            // iced font names require static storage; intern each applied family once.
            let name = *families
                .entry(name.to_owned())
                .or_insert_with(|| Box::leak(name.to_owned().into_boxed_str()));
            iced::font::Family::Name(name)
        }
    };
    iced::Font {
        family,
        ..Default::default()
    }
}

pub fn label<'a>(
    value: impl iced::advanced::text::IntoFragment<'a>,
    font: iced::Font,
    size: f32,
) -> iced::widget::Text<'a> {
    iced::widget::text(value)
        .font(font)
        .size(size)
}

pub fn color(value: u32) -> Color {
    Color::from_rgb8((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

pub fn rgba(value: u32) -> Color {
    let mut color = color(value >> 8);
    color.a = (value & 255) as f32 / 255.0;
    color
}

pub fn theme(config: &ThemeConfig, system_dark: bool) -> Theme {
    let palette = match config.mode {
        ThemeMode::Dark => &config.dark,
        ThemeMode::Light => &config.light,
        ThemeMode::System if system_dark => &config.dark,
        ThemeMode::System => &config.light,
    };
    Theme::custom_with_fn(
        "Ghost".to_owned(),
        iced::theme::Palette {
            background: color(palette.base00),
            text: color(palette.base05),
            primary: color(palette.base0d),
            success: color(palette.base0b),
            warning: color(palette.base0a),
            danger: color(palette.base08),
        },
        |simple| {
            let pair = |background, foreground| Pair {
                color: color(background),
                text: color(foreground),
            };
            let selection = pair(palette.base07, palette.base00);
            let mut extended = Extended::generate(simple);
            extended.background = Background {
                base: pair(palette.base00, palette.base05),
                weakest: pair(palette.base00, palette.base05),
                weaker: pair(palette.base01, palette.base05),
                weak: pair(palette.base01, palette.base05),
                neutral: selection,
                strong: pair(palette.base03, palette.base05),
                stronger: pair(palette.base04, palette.base05),
                strongest: pair(palette.base06, palette.base00),
            };
            extended.secondary = Secondary {
                base: contrast_pair(palette.base04, palette.base00, palette.base07),
                weak: pair(palette.base01, palette.base05),
                strong: contrast_pair(palette.base02, palette.base00, palette.base07),
            };
            // Accent variants retain iced's hover/pressed behavior; their text
            // comes from the configured palette instead of generated black/white.
            for pair in [
                &mut extended.primary.base,
                &mut extended.primary.weak,
                &mut extended.primary.strong,
                &mut extended.success.base,
                &mut extended.success.weak,
                &mut extended.success.strong,
                &mut extended.warning.base,
                &mut extended.warning.weak,
                &mut extended.warning.strong,
                &mut extended.danger.base,
                &mut extended.danger.weak,
                &mut extended.danger.strong,
            ] {
                pair.text = contrast_text(
                    pair.color,
                    color(palette.base00),
                    color(palette.base07),
                );
            }
            extended
        },
    )
}

fn contrast_text(background: Color, first: Color, second: Color) -> Color {
    if background.relative_contrast(first) >= background.relative_contrast(second) {
        first
    } else {
        second
    }
}

fn contrast_pair(background: u32, first: u32, second: u32) -> Pair {
    let background = color(background);
    Pair {
        color: background,
        text: contrast_text(background, color(first), color(second)),
    }
}

pub fn panel(theme: &Theme, opacity: f32) -> container::Style {
    surface(theme, opacity)
}

pub fn text_input(
    theme: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let mut style = iced::widget::text_input::default(theme, status);
    style.selection = theme
        .extended_palette()
        .secondary
        .strong
        .color;
    style.placeholder = theme
        .extended_palette()
        .background
        .strong
        .color;
    if matches!(status, iced::widget::text_input::Status::Disabled) {
        style.value = theme.extended_palette().secondary.base.color;
    }
    style
}

pub fn surface(theme: &Theme, opacity: f32) -> container::Style {
    let mut background = theme.palette().background;
    background.a = opacity;
    container::Style::default()
        .background(background)
        .color(theme.palette().text)
}

pub fn search_window(theme: &Theme, opacity: f32) -> container::Style {
    surface(theme, opacity).border(iced::Border {
        color: theme
            .extended_palette()
            .background
            .neutral
            .color,
        width: 1.0,
        radius: 4.0.into(),
    })
}

pub fn search_input(
    theme: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let mut style = text_input(theme, status);
    style.border.color = theme
        .extended_palette()
        .background
        .neutral
        .color;
    style.border.radius = 4.0.into();
    style
}

pub fn search_item(
    theme: &Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    let mut style = iced::widget::button::text(theme, status);
    style.text_color = theme.palette().text;
    style.background = if selected {
        Some(
            theme
                .extended_palette()
                .secondary
                .strong
                .color
                .into(),
        )
    } else if matches!(
        status,
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
    ) {
        Some(
            theme
                .extended_palette()
                .background
                .weak
                .color
                .into(),
        )
    } else {
        None
    };
    style.border.radius = 4.0.into();
    style
}

pub fn selected(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let pair = theme.extended_palette().background.neutral;
    let mut style = iced::widget::button::text(theme, status);
    style.background = Some(pair.color.into());
    style.text_color = pair.text;
    style.border = iced::Border {
        color: pair.color,
        width: 1.0,
        ..style.border
    };
    if matches!(status, iced::widget::button::Status::Disabled) {
        style.text_color = style.text_color.scale_alpha(0.5);
    }
    style
}

pub fn plain(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::text(theme, status)
}

pub fn search_scrollable(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let mut style = scrollable::default(theme, status);
    let (horizontal, vertical) = match status {
        scrollable::Status::Hovered {
            is_horizontal_scrollbar_hovered,
            is_vertical_scrollbar_hovered,
            ..
        } => (
            is_horizontal_scrollbar_hovered,
            is_vertical_scrollbar_hovered,
        ),
        scrollable::Status::Dragged {
            is_horizontal_scrollbar_dragged,
            is_vertical_scrollbar_dragged,
            ..
        } => (
            is_horizontal_scrollbar_dragged,
            is_vertical_scrollbar_dragged,
        ),
        scrollable::Status::Active { .. } => (false, false),
    };
    let background = theme.palette().danger.into();
    if horizontal {
        style.horizontal_rail.scroller.background = background;
    }
    if vertical {
        style.vertical_rail.scroller.background = background;
    }
    style
}

pub fn virtual_list<'a, Message: Clone + 'a>(
    count: usize,
    offset: f32,
    viewport_height: f32,
    row_height: f32,
    id: &'static str,
    on_scroll: impl Fn(scrollable::Viewport) -> Message + 'a,
    mut row: impl FnMut(usize) -> Element<'a, Message>,
) -> Element<'a, Message> {
    let start = ((offset.max(0.0) / row_height) as usize)
        .saturating_sub(1)
        .min(count);
    let end = (start + (viewport_height / row_height).ceil() as usize + 3).min(count);
    let mut content = column![Space::new().height(start as f32 * row_height)];
    for index in start..end {
        content = content.push(
            container(row(index))
                .height(row_height)
                .width(Fill)
                .clip(true),
        );
    }
    content = content.push(Space::new().height((count - end) as f32 * row_height));
    scrollable(content)
        .style(search_scrollable)
        .id(id)
        .on_scroll(on_scroll)
        .height(Fill)
        .into()
}

pub fn reveal_list_row(id: &'static str, top: f32, height: f32) -> iced::Task<f32> {
    iced::advanced::widget::operate(RevealRow {
        list: id.into(),
        target: None,
        row: Some((top, height)),
        viewport: None,
    })
}

pub fn reveal_list_item(id: &'static str, target: &'static str) -> iced::Task<f32> {
    iced::advanced::widget::operate(RevealRow {
        list: id.into(),
        target: Some(target.into()),
        row: None,
        viewport: None,
    })
}

struct RevealRow {
    list: iced::advanced::widget::Id,
    target: Option<iced::advanced::widget::Id>,
    row: Option<(f32, f32)>,
    viewport: Option<(iced::Rectangle, iced::Rectangle, f32)>,
}

impl iced::advanced::widget::Operation<f32> for RevealRow {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation<f32>),
    ) {
        operate(self);
    }

    fn container(
        &mut self,
        id: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
    ) {
        if self.target.is_some() && id == self.target.as_ref() {
            if let Some((_, content, _)) = self.viewport {
                self.row = Some((bounds.y - content.y, bounds.height));
            }
        }
    }

    fn scrollable(
        &mut self,
        id: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        content: iced::Rectangle,
        translation: iced::Vector,
        _state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if id == Some(&self.list) {
            self.viewport = Some((bounds, content, translation.y));
        }
    }

    fn finish(&self) -> iced::advanced::widget::operation::Outcome<f32> {
        use iced::advanced::widget::operation::Outcome;
        let (Some((top, height)), Some((bounds, content, offset))) =
            (self.row, self.viewport)
        else {
            return Outcome::None;
        };
        let next = visible_row_offset(offset, bounds.height, top, height)
            .clamp(0.0, (content.height - bounds.height).max(0.0));
        if (next - offset).abs() > 0.5 {
            Outcome::Some(next)
        } else {
            Outcome::None
        }
    }
}

fn visible_row_offset(offset: f32, viewport: f32, top: f32, height: f32) -> f32 {
    if top < offset || height > viewport {
        top
    } else if top + height > offset + viewport {
        top + height - viewport
    } else {
        offset
    }
}

pub fn selection(current: Option<usize>, count: usize, forward: bool) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some(match current.filter(|index| *index < count) {
        None => 0,
        Some(index) if forward => (index + 1) % count,
        Some(0) => count - 1,
        Some(index) => index - 1,
    })
}

#[cfg(test)]
mod tests {
    use ghost_shell_config::Base16Config;

    use super::*;

    #[test]
    fn keyboard_navigation_scrolls_only_past_visible_edges() {
        assert_eq!(visible_row_offset(0.0, 360.0, 48.0, 48.0), 0.0);
        assert_eq!(visible_row_offset(0.0, 360.0, 336.0, 48.0), 24.0);
        assert_eq!(visible_row_offset(96.0, 360.0, 48.0, 48.0), 48.0);
        assert_eq!(visible_row_offset(96.0, 360.0, 128.0, 64.0), 96.0);
        assert_eq!(visible_row_offset(96.0, 360.0, 416.0, 64.0), 120.0);
        assert_eq!(visible_row_offset(96.0, 360.0, 0.0, 32.0), 0.0);
        assert_eq!(visible_row_offset(0.0, 360.0, 480.0, 32.0), 152.0);
    }

    fn assert_mapping(theme: &Theme, source: &Base16Config) {
        let palette = theme.palette();
        assert_eq!(palette.background, color(source.base00));
        assert_eq!(palette.text, color(source.base05));
        assert_eq!(palette.primary, color(source.base0d));
        assert_eq!(palette.success, color(source.base0b));
        assert_eq!(palette.warning, color(source.base0a));
        assert_eq!(palette.danger, color(source.base08));
        let extended = theme.extended_palette();
        for (pair, value) in [
            (extended.background.base, source.base00),
            (extended.background.weakest, source.base00),
            (extended.background.weaker, source.base01),
            (extended.background.weak, source.base01),
            (extended.background.neutral, source.base07),
            (extended.background.strong, source.base03),
            (extended.background.stronger, source.base04),
            (extended.background.strongest, source.base06),
            (extended.secondary.base, source.base04),
            (extended.secondary.weak, source.base01),
            (extended.secondary.strong, source.base02),
        ] {
            assert_eq!(pair.color, color(value));
        }
        for pair in [
            extended.background.neutral,
            extended.primary.base,
            extended.primary.weak,
            extended.primary.strong,
            extended.success.base,
            extended.warning.base,
            extended.danger.base,
        ] {
            assert!(
                pair.text == color(source.base00) || pair.text == color(source.base07)
            );
            assert!(
                pair.color.relative_contrast(pair.text)
                    >= pair
                        .color
                        .relative_contrast(color(source.base00))
            );
            assert!(
                pair.color.relative_contrast(pair.text)
                    >= pair
                        .color
                        .relative_contrast(color(source.base07))
            );
        }
    }

    #[test]
    fn polarity_selects_the_whole_base16_palette() {
        let mut config = ThemeConfig::default();
        config.dark.base01 = 0x212830;
        config.dark.base02 = 0x536475;
        config.dark.base07 = 0xfefdfc;
        for (mode, system_dark, dark) in [
            (ThemeMode::Dark, false, true),
            (ThemeMode::Light, true, false),
            (ThemeMode::System, true, true),
            (ThemeMode::System, false, false),
        ] {
            config.mode = mode;
            let theme = theme(&config, system_dark);
            assert_mapping(&theme, if dark { &config.dark } else { &config.light });
            assert_eq!(theme.extended_palette().is_dark, dark);
        }
    }

    #[test]
    fn surfaces_and_selection_use_their_semantic_roles() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let config = ThemeConfig {
                mode,
                ..ThemeConfig::default()
            };
            let theme = theme(&config, false);
            let source = if matches!(mode, ThemeMode::Dark) {
                &config.dark
            } else {
                &config.light
            };
            assert_eq!(
                surface(&theme, 0.8).background,
                Some(
                    Color {
                        a: 0.8,
                        ..color(source.base00)
                    }
                    .into()
                )
            );
            assert_eq!(
                panel(&theme, 0.8).background,
                Some(
                    Color {
                        a: 0.8,
                        ..color(source.base00)
                    }
                    .into()
                )
            );
            for status in [
                iced::widget::button::Status::Active,
                iced::widget::button::Status::Hovered,
                iced::widget::button::Status::Pressed,
            ] {
                let style = selected(&theme, status);
                assert_eq!(style.background, Some(color(source.base07).into()));
                assert_eq!(style.border.color, color(source.base07));
                assert_eq!(style.border.width, 1.0);
                assert_eq!(style.text_color, color(source.base00));
                assert_eq!(
                    style.text_color,
                    theme
                        .extended_palette()
                        .background
                        .neutral
                        .text
                );
            }
            let input = text_input(&theme, iced::widget::text_input::Status::Active);
            assert_eq!(input.selection, color(source.base02));
            assert_eq!(input.placeholder, color(source.base03));
            assert_eq!(input.value, color(source.base05));
        }
    }
}

pub fn menu_surface(theme: &Theme) -> container::Style {
    surface(theme, 1.0).border(iced::Border {
        color: theme
            .extended_palette()
            .background
            .strong
            .color,
        width: 1.0,
        radius: 6.0.into(),
    })
}

pub fn menu_item(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    if matches!(
        status,
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
    ) {
        let mut style = selected(theme, status);
        style.border.radius = 4.0.into();
        style
    } else {
        plain(theme, status)
    }
}
