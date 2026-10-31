use ghost_shell_audio::AudioState;
use ghost_shell_config::BarConfig;
use ghost_shell_niri::NiriState;
use ghost_shell_widget_clock::Clock;
use ghost_shell_widget_power::Power;
use ghost_shell_widget_tray::Tray;
use iced::{
    Element, Fill,
    widget::{Space, container, row},
};

#[derive(Debug, Clone)]
pub enum Message {
    Menu(ghost_shell_widget_menu::Message),
    Audio(ghost_shell_widget_audio::Message),
    Tray(ghost_shell_widget_tray::TrayMessage),
    Decorations(Vec<ghost_shell_theme::bar::Decoration>),
    Styling(ghost_shell_widget_styling::Message),
}

#[derive(Default)]
pub struct Bar {
    pub tray: Tray,
    pub clock: Clock,
    pub power: Power,
}

impl Bar {
    pub fn view<'a>(
        &'a self,
        config: &'a BarConfig,
        general: &ghost_shell_config::GeneralConfig,
        output: &str,
        niri: &'a NiriState,
        audio: Option<&'a AudioState>,
    ) -> Element<'a, Message> {
        let style = ghost_shell_theme::bar::Style {
            font: ghost_shell_theme::font(&general.font_family),
            font_size: general.font_size,
            ..ghost_shell_theme::bar::Style::from(config)
        };
        let start = row![
            ghost_shell_widget_menu::view(style).map(Message::Menu),
            ghost_shell_widget_workspaces::view(output, niri, style),
        ]
        .spacing(0)
        .align_y(iced::Center)
        .width(Fill);

        let end = row![
            Space::new().width(Fill),
            self.tray.view(style).map(Message::Tray),
            ghost_shell_widget_audio::view(audio, style).map(Message::Audio),
            self.power.view(style),
            ghost_shell_theme::bar::outer(
                ghost_shell_theme::bar::inner(
                    self.clock.view(style.font, style.font_size),
                    style
                ),
                style
            ),
        ]
        .spacing(0)
        .align_y(iced::Center)
        .width(Fill)
        .height(Fill);

        let center =
            container(ghost_shell_widget_focus::view(niri, style)).center_x(Fill);

        ghost_shell_theme::bar::decorated(
            container(row![start, center, end].align_y(iced::Center))
                .padding([0, 9])
                .width(Fill)
                .height(Fill),
            style,
            Message::Decorations,
        )
    }
}
