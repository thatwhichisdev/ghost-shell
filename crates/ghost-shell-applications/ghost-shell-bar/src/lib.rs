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
        output: &str,
        niri: &'a NiriState,
        audio: Option<&'a AudioState>,
    ) -> Element<'a, Message> {
        let start = row![
            ghost_shell_widget_menu::view().map(Message::Menu),
            ghost_shell_widget_workspaces::view(output, niri),
        ]
        .spacing(8)
        .align_y(iced::Center)
        .width(Fill);

        let end = row![
            Space::new().width(Fill),
            self.tray.view().map(Message::Tray),
            ghost_shell_widget_audio::view(audio).map(Message::Audio),
            self.power.view(),
            ghost_shell_theme::bar::outer(ghost_shell_theme::bar::inner(
                self.clock.view()
            )),
        ]
        .spacing(0)
        .align_y(iced::Center)
        .width(Fill)
        .height(Fill);

        let center = ghost_shell_widget_focus::view(niri);

        container(row![start, center, end].align_y(iced::Center))
            .padding([0, 9])
            .width(Fill)
            .height(Fill)
            .style(move |theme| {
                ghost_shell_theme::panel(theme, config.background_opacity())
            })
            .into()
    }
}
