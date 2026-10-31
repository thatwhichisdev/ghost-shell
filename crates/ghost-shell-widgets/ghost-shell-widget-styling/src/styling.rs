use ghost_shell_config::{BarAppConfig, BarLayout, Base16Config, ThemeMode};
use iced::{
    Element, Fill,
    widget::{button, column, container, row, scrollable, slider, text, text_input},
};

#[derive(Clone, Debug)]
pub enum Message {
    Close,
    Reset,
    Copy,
    Number(Number, f32),
    Islands(bool),
    Blur(bool),
    Mode(ThemeMode),
    Palette(bool),
    Color(usize, String),
    Icons(String),
    ApplyIcons,
    Font(String),
    ApplyFont,
}

#[derive(Clone, Copy, Debug)]
pub enum Number {
    Height,
    Zone,
    Opacity,
    Radius,
    WidgetHeight,
    IconSize,
    Padding,
    HorizontalMargin,
    VerticalMargin,
    FontSize,
}

pub struct Styling {
    dark: bool,
    colors: [String; 16],
    icons: String,
    font: String,
    pub error: Option<String>,
    pub notice: Option<String>,
}

impl Styling {
    pub fn new(config: &BarAppConfig) -> Self {
        let dark = !matches!(config.theme.mode, ThemeMode::Light);
        Self {
            dark,
            colors: palette_values(if dark {
                &config.theme.dark
            } else {
                &config.theme.light
            })
            .map(|value| format!("#{value:06x}")),
            icons: config
                .theme
                .icons
                .clone()
                .unwrap_or_default(),
            font: config.general.font_family.clone(),
            error: None,
            notice: None,
        }
    }

    pub fn update(&mut self, message: Message, config: &mut BarAppConfig) -> bool {
        self.notice = None;
        match message {
            Message::Number(number, value) => {
                if !value.is_finite() {
                    return false;
                }
                let bar = &mut config.bar;
                match number {
                    Number::Height => bar.height = value.clamp(1.0, 16384.0),
                    Number::Zone => bar.exclusive_zone = value.clamp(0.0, 16384.0),
                    Number::Opacity => bar.decoration.opacity = value.clamp(0.0, 1.0),
                    Number::Radius => bar.decoration.radius = value.clamp(0.0, 16384.0),
                    Number::WidgetHeight => {
                        bar.widgets.height = value.clamp(1.0, 16384.0)
                    }
                    Number::IconSize => bar.widgets.icon_size = value.clamp(1.0, 16384.0),
                    Number::Padding => {
                        bar.widgets.padding_horizontal = value.clamp(0.0, 16384.0)
                    }
                    Number::HorizontalMargin => {
                        bar.widgets.margin_horizontal = value.clamp(0.0, 16384.0)
                    }
                    Number::VerticalMargin => {
                        bar.widgets.margin_vertical = value.clamp(0.0, 16384.0)
                    }
                    Number::FontSize => {
                        config.general.font_size = value.clamp(1.0, 16384.0)
                    }
                }
            }
            Message::Islands(value) => {
                config.bar.decoration.layout = if value {
                    BarLayout::Islands
                } else {
                    BarLayout::Continuous
                }
            }
            Message::Blur(value) => config.bar.decoration.blur = value,
            Message::Mode(value) => config.theme.mode = value,
            Message::Palette(dark) => {
                self.dark = dark;
                self.colors = palette_values(if dark {
                    &config.theme.dark
                } else {
                    &config.theme.light
                })
                .map(|value| format!("#{value:06x}"));
                self.error = None;
                return false;
            }
            Message::Color(index, value) => {
                let Some(input) = self.colors.get_mut(index) else {
                    return false;
                };
                *input = value.clone();
                let Some(color) = parse_color(&value) else {
                    self.error =
                        Some("Colors need #RRGGBB (six hexadecimal digits).".into());
                    return false;
                };
                let palette = if self.dark {
                    &mut config.theme.dark
                } else {
                    &mut config.theme.light
                };
                if let Some(slot) = palette_slots(palette).into_iter().nth(index) {
                    *slot = color;
                }
                self.error = self
                    .colors
                    .iter()
                    .any(|value| parse_color(value).is_none())
                    .then(|| "Colors need #RRGGBB (six hexadecimal digits).".into());
            }
            Message::Icons(value) => {
                self.icons = value;
                return false;
            }
            Message::Font(value) => {
                self.font = value;
                return false;
            }
            Message::ApplyIcons => {
                config.theme.icons =
                    (!self.icons.trim().is_empty()).then(|| self.icons.trim().to_owned())
            }
            Message::ApplyFont => {
                if self.font.trim().is_empty() {
                    self.error = Some("Font family cannot be empty.".into());
                    return false;
                }
                config.general.font_family = self.font.trim().into();
                if self
                    .colors
                    .iter()
                    .all(|value| parse_color(value).is_some())
                {
                    self.error = None;
                }
            }
            Message::Close | Message::Reset | Message::Copy => {
                return false;
            }
        }
        true
    }

    pub fn view<'a>(
        &'a self,
        config: &'a BarAppConfig,
        reload_error: Option<&'a str>,
    ) -> Element<'a, Message> {
        let bar = &config.bar;
        let islands = bar.decoration.layout == BarLayout::Islands;
        let mut feedback = column![].spacing(6);
        for error in [self.error.as_deref(), reload_error]
            .into_iter()
            .flatten()
        {
            feedback = feedback.push(text(error).style(iced::widget::text::danger));
        }
        if let Some(notice) = &self.notice {
            feedback = feedback.push(text(notice));
        }
        let mut controls = column![
            row![
                text("Appearance").size(18),
                iced::widget::Space::new().width(Fill),
                button("Close").on_press(Message::Close)
            ]
            .spacing(8),
            text("Live preview · Copy Nix to persist your settings").size(12),
            feedback,
            row![
                button("Reset from file").on_press(Message::Reset),
                button("Copy Nix").on_press(Message::Copy)
            ]
            .spacing(8),
            row![
                toggle("Continuous", !islands, Message::Islands(false)),
                toggle("Islands", islands, Message::Islands(true))
            ]
            .spacing(8),
            toggle(
                "Compositor blur",
                bar.decoration.blur,
                Message::Blur(!bar.decoration.blur)
            ),
            number("Bar height", bar.height, 1.0, 100.0, Number::Height),
            number(
                "Reserved space",
                bar.exclusive_zone,
                0.0,
                100.0,
                Number::Zone
            ),
            number(
                "Background opacity",
                bar.decoration.opacity,
                0.0,
                1.0,
                Number::Opacity
            ),
            number(
                "Corner radius",
                bar.decoration.radius,
                0.0,
                32.0,
                Number::Radius
            ),
            text("Widget dimensions"),
            number(
                "Inner height",
                bar.widgets.height,
                1.0,
                64.0,
                Number::WidgetHeight
            ),
            number(
                "Icon size",
                bar.widgets.icon_size,
                1.0,
                64.0,
                Number::IconSize
            ),
            number(
                "Horizontal padding",
                bar.widgets.padding_horizontal,
                0.0,
                32.0,
                Number::Padding
            ),
            number(
                "Horizontal margin",
                bar.widgets.margin_horizontal,
                0.0,
                32.0,
                Number::HorizontalMargin
            ),
            number(
                "Vertical margin",
                bar.widgets.margin_vertical,
                0.0,
                32.0,
                Number::VerticalMargin
            ),
            number(
                "Font size",
                config.general.font_size,
                1.0,
                32.0,
                Number::FontSize
            ),
            row![
                text_input("Font family", &self.font)
                    .style(ghost_shell_theme::search_input)
                    .on_input(Message::Font)
                    .on_submit(Message::ApplyFont),
                button("Apply font").on_press(Message::ApplyFont)
            ]
            .spacing(8),
            text("Theme"),
            row![
                toggle(
                    "Dark",
                    config.theme.mode == ThemeMode::Dark,
                    Message::Mode(ThemeMode::Dark)
                ),
                toggle(
                    "Light",
                    config.theme.mode == ThemeMode::Light,
                    Message::Mode(ThemeMode::Light)
                ),
                toggle(
                    "System",
                    config.theme.mode == ThemeMode::System,
                    Message::Mode(ThemeMode::System)
                )
            ]
            .spacing(8),
            row![
                text_input("Icon theme (empty = automatic)", &self.icons)
                    .style(ghost_shell_theme::search_input)
                    .on_input(Message::Icons)
                    .on_submit(Message::ApplyIcons),
                button("Apply icons").on_press(Message::ApplyIcons)
            ]
            .spacing(8),
            row![
                toggle("Edit dark palette", self.dark, Message::Palette(true)),
                toggle("Edit light palette", !self.dark, Message::Palette(false))
            ]
            .spacing(8),
        ]
        .spacing(10);
        for (index, value) in self.colors.iter().enumerate() {
            controls = controls.push(
                row![
                    text(format!("base{index:02X}")).width(70),
                    container(
                        iced::widget::Space::new()
                            .width(20)
                            .height(20)
                    )
                    .style(move |theme: &iced::Theme| {
                        container::Style::default().background(
                            parse_color(value)
                                .map(ghost_shell_theme::color)
                                .unwrap_or(theme.palette().background),
                        )
                    }),
                    text_input("#RRGGBB", value)
                        .style(ghost_shell_theme::search_input)
                        .on_input(move |value| Message::Color(index, value)),
                ]
                .spacing(10)
                .align_y(iced::Center),
            );
        }
        container(scrollable(controls).style(ghost_shell_theme::search_scrollable))
            .padding(12)
            .width(Fill)
            .height(Fill)
            .style(|theme| ghost_shell_theme::search_window(theme, 1.0))
            .into()
    }
}

fn toggle<'a>(label: &'a str, selected: bool, message: Message) -> Element<'a, Message> {
    button(text(label))
        .on_press(message)
        .style(move |theme, status| {
            ghost_shell_theme::search_item(theme, status, selected)
        })
        .into()
}

fn number(
    label: &'static str,
    value: f32,
    minimum: f32,
    maximum: f32,
    number: Number,
) -> Element<'static, Message> {
    let step: f32 = if matches!(number, Number::Opacity) {
        0.01
    } else {
        1.0
    };
    column![
        row![text(label).width(Fill), text(format!("{value:.2}"))],
        slider(minimum..=maximum.max(value), value, move |value| {
            Message::Number(number, value)
        })
        .step(step)
    ]
    .spacing(3)
    .into()
}

fn parse_color(value: &str) -> Option<u32> {
    let digits = value.strip_prefix('#').filter(|digits| {
        digits.len() == 6
            && digits
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
    })?;
    u32::from_str_radix(digits, 16).ok()
}

fn palette_slots(palette: &mut Base16Config) -> [&mut u32; 16] {
    [
        &mut palette.base00,
        &mut palette.base01,
        &mut palette.base02,
        &mut palette.base03,
        &mut palette.base04,
        &mut palette.base05,
        &mut palette.base06,
        &mut palette.base07,
        &mut palette.base08,
        &mut palette.base09,
        &mut palette.base0a,
        &mut palette.base0b,
        &mut palette.base0c,
        &mut palette.base0d,
        &mut palette.base0e,
        &mut palette.base0f,
    ]
}

fn palette_values(palette: &Base16Config) -> [u32; 16] {
    [
        palette.base00,
        palette.base01,
        palette.base02,
        palette.base03,
        palette.base04,
        palette.base05,
        palette.base06,
        palette.base07,
        palette.base08,
        palette.base09,
        palette.base0a,
        palette.base0b,
        palette.base0c,
        palette.base0d,
        palette.base0e,
        palette.base0f,
    ]
}

pub fn export_nix(config: &BarAppConfig) -> Result<String, serde_json::Error> {
    let mut value = serde_json::json!({ "general": config.general, "bar": config.bar, "theme": config.theme });
    for (name, palette) in [("dark", &config.theme.dark), ("light", &config.theme.light)]
    {
        if let Some(object) = value
            .get_mut("theme")
            .and_then(|theme| theme.get_mut(name))
            .and_then(serde_json::Value::as_object_mut)
        {
            for (index, color) in palette_values(palette)
                .into_iter()
                .enumerate()
            {
                object.insert(format!("base{index:02X}"), format!("#{color:06x}").into());
            }
        }
    }
    Ok(format!(
        "programs.ghost-shell.settings = {};\n",
        nix_value(&value, 0)?
    ))
}

fn nix_value(
    value: &serde_json::Value,
    indent: usize,
) -> Result<String, serde_json::Error> {
    match value {
        serde_json::Value::Object(object) => {
            let mut output = String::from("{\n");
            for (key, value) in object
                .iter()
                .filter(|(_, value)| !value.is_null())
            {
                output.push_str(&format!(
                    "{}{} = {};\n",
                    " ".repeat(indent + 2),
                    nix_string(key)?,
                    nix_value(value, indent + 2)?
                ));
            }
            output.push_str(&format!("{}}}", " ".repeat(indent)));
            Ok(output)
        }
        serde_json::Value::String(value) => nix_string(value),
        serde_json::Value::Number(number) if number.is_f64() => match number.as_f64() {
            Some(value) => Ok(format!("{}", value as f32)),
            None => serde_json::to_string(value),
        },
        _ => serde_json::to_string(value),
    }
}

fn nix_string(value: &str) -> Result<String, serde_json::Error> {
    Ok(serde_json::to_string(value)?.replace("${", "\\${"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_palette_edit_preserves_the_last_valid_color() {
        let mut config = BarAppConfig::default();
        let mut editor = Styling::new(&config);
        let original = config.theme.dark.base00;
        assert!(!editor.update(Message::Color(0, "#12".into()), &mut config));
        assert_eq!(config.theme.dark.base00, original);
        assert!(editor.error.is_some());
        assert!(editor.update(Message::Color(0, "#123abc".into()), &mut config));
        assert_eq!(config.theme.dark.base00, 0x123abc);
        assert!(editor.error.is_none());
        assert!(!editor.update(Message::Color(100, "#123abc".into()), &mut config));
    }

    #[test]
    fn palette_edits_do_not_change_the_other_polarity() {
        let mut config = BarAppConfig::default();
        let mut editor = Styling::new(&config);
        let original = config.theme.dark.clone();
        editor.update(Message::Palette(false), &mut config);
        editor.update(Message::Color(10, "#abcdef".into()), &mut config);
        assert_eq!(config.theme.dark, original);
        assert_eq!(config.theme.light.base0a, 0xabcdef);
    }

    #[test]
    fn export_preserves_output_names_and_escapes_nix_interpolation()
    -> Result<(), serde_json::Error> {
        let mut config = BarAppConfig::default();
        config.bar.decoration.opacity = 0.8;
        config.general.font_family = "Font ${name} \"Quoted\"".into();
        config
            .bar
            .outputs
            .insert("DP-1".into(), Default::default());
        let output = export_nix(&config)?;
        assert!(output.contains("\\${name}"));
        assert!(output.contains("\\\"Quoted\\\""));
        assert!(output.contains("\"DP-1\" = {"));
        assert!(output.contains("\"base0A\" = \"#e09b13\";"));
        assert!(!output.contains("\"icons\" = null"));
        assert!(!output.contains("wallpaper"));
        assert!(output.contains("\"opacity\" = 0.8;"));
        Ok(())
    }
}
