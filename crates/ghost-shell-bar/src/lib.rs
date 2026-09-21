pub mod bar;

use std::collections::HashMap;

pub use bar::*;
use ghost_shell_config::AppConfig;
use ghost_shell_gpui::{
    App, AppContext, BorrowAppContext, Entity, Global, accesskit::Uuid,
};
use ghost_shell_widget_clock::ClockWidget;
use ghost_shell_widget_focus::FocusWidget;
use ghost_shell_widget_menu::MenuWidget;
use ghost_shell_widget_power::PowerWidget;
use ghost_shell_widget_tray::TrayWidget;
use ghost_shell_widget_workspaces::WorkspacesWidget;

struct BarManager {
    bars: HashMap<Uuid, Entity<Bar>>,
    menu: Entity<MenuWidget>,
    power: Entity<PowerWidget>,
    clock: Entity<ClockWidget>,
    focus: Entity<FocusWidget>,
    tray: Entity<TrayWidget>,
}

impl Global for BarManager {}

impl BarManager {
    fn reconcile(&mut self, cx: &mut App) {
        let displays = cx.displays();
        let desired: HashMap<_, _> = cx
            .global::<AppConfig>()
            .bars
            .iter()
            .filter_map(|(output, config)| {
                let identity = Uuid::new_v5(&Uuid::NAMESPACE_DNS, output.as_bytes());
                displays
                    .iter()
                    .find(|display| {
                        display
                            .uuid()
                            .is_ok_and(|uuid| uuid == identity)
                    })
                    .map(|display| (identity, (display.clone(), config.clone())))
            })
            .collect();

        self.bars.retain(|identity, bar| {
            if desired
                .get(identity)
                .is_some_and(|(display, _)| bar.read(cx).is_current(display.as_ref(), cx))
            {
                return true;
            }
            if let Err(error) = bar.update(cx, |bar, cx| bar.close(cx)) {
                log::error!("Failed to close bar on {identity}: {error:#}");
            }
            false
        });

        for (identity, (display, config)) in desired {
            if self.bars.contains_key(&identity) {
                continue;
            }
            let workspaces = cx.new(|cx| WorkspacesWidget::new(cx, identity));
            let widgets = Widgets {
                menu: self.menu.clone(),
                power: self.power.clone(),
                clock: self.clock.clone(),
                focus: self.focus.clone(),
                tray: self.tray.clone(),
                workspaces,
            };
            let bar = Bar::new(cx, config, widgets, display);
            match bar.update(cx, |bar, cx| bar.open(cx)) {
                Ok(()) => {
                    self.bars.insert(identity, bar);
                }
                Err(error) => log::error!("Failed to open bar on {identity}: {error:#}"),
            }
        }
    }
}

pub fn init(cx: &mut App) {
    let power = match PowerWidget::try_new() {
        Ok(power) => cx.new(|_| power),
        Err(error) => {
            log::error!("Failed to initialize bar power widget: {error:#}");
            return;
        }
    };
    let mut manager = BarManager {
        bars: HashMap::new(),
        menu: cx.new(|_| MenuWidget {}),
        power,
        clock: cx.new(ClockWidget::new),
        focus: cx.new(FocusWidget::new),
        tray: cx.new(TrayWidget::new),
    };
    manager.reconcile(cx);
    cx.set_global(manager);
    cx.on_displays_changed(|cx| {
        cx.update_global::<BarManager, _>(|manager, cx| manager.reconcile(cx));
    })
    .detach();
}
