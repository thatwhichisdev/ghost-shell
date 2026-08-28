pub mod bar;

pub use bar::*;

use gpui::{App, AppContext, accesskit::Uuid};

use ghost_shell_config::AppConfig;
use ghost_shell_widget_clock::ClockWidget;
use ghost_shell_widget_focus::FocusWidget;
use ghost_shell_widget_menu::MenuWidget;
use ghost_shell_widget_power::PowerWidget;
use ghost_shell_widget_tray::TrayWidget;
use ghost_shell_widget_workspaces::WorkspacesWidget;

pub fn init(cx: &mut App) {
    let config = cx.global::<AppConfig>().clone();

    let menu = cx.new(|_cx| MenuWidget {});
    let power = cx.new(|_cx| PowerWidget::try_new().unwrap());
    let clock = cx.new(ClockWidget::new);
    let focus = cx.new(FocusWidget::new);
    let tray = cx.new(TrayWidget::new);

    config.bars.into_iter().for_each(|(output, bar_config)| {
        let id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, output.as_bytes());
        let display = cx
            .displays()
            .iter()
            .find(|display| display.uuid().is_ok_and(|uuid| uuid == id))
            .cloned()
            .unwrap(); // for now display should be always present in the config, will change later

        let workspaces = cx.new(|cx| WorkspacesWidget::new(cx, id));
        let widgets = Widgets {
            menu: menu.clone(),
            workspaces,
            focus: focus.clone(),
            tray: tray.clone(),
            power: power.clone(),
            clock: clock.clone(),
        };

        let bar = Bar::new(cx, bar_config, widgets, display.clone());

        bar.update(cx, |bar, cx| {
            bar.open(cx);
        });
    });
}
