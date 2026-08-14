use anyhow::{Context as _, Result};
use ghost_shell_actions::{LauncherClose, LauncherToggle};
use ghost_shell_app::GhostShell;
use gpui::{
    App, AppContext as _, BorrowAppContext as _, Bounds, Global, KeyBinding,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions, px, size,
};
use gpui_component::Root;

use crate::{actions::EntrySpawn, view::View};

pub fn init(cx: &mut App) {
    cx.set_global(Launcher::new());

    cx.bind_keys([
        KeyBinding::new("escape", LauncherClose, Some("launcher")),
        KeyBinding::new("enter", EntrySpawn, Some("launcher")),
    ]);

    cx.on_action(|_: &LauncherClose, cx| {
        cx.defer(|cx| {
            cx.update_global::<Launcher, _>(|launcher, cx| {
                match launcher.close(cx) {
                    Ok(()) => {}
                    Err(err) => eprintln!("Failed to close launcher {err:#}"),
                }
            });
        });
    });

    cx.on_action(|_: &LauncherToggle, cx| {
        cx.update_global::<Launcher, _>(|launcher, cx| {
            match launcher.toggle(cx) {
                Ok(()) => {}
                Err(err) => eprintln!("Failed to toggle launcher {err:#}"),
            }
        });
    });
}

pub struct Launcher {
    handle: Option<WindowHandle<Root>>,
}

impl Global for Launcher {}

impl Launcher {
    #[must_use]
    pub fn new() -> Self {
        Self { handle: None }
    }

    pub fn toggle(&mut self, cx: &mut App) -> Result<()> {
        if self.handle.is_none() {
            self.open(cx)
        } else {
            self.close(cx)
        }
    }

    pub fn open(&mut self, cx: &mut App) -> Result<()> {
        let output = cx.global::<GhostShell>().get_output();
        let window_bounds = Bounds::centered(
            Some(output.display.id()),
            size(px(540.0), px(450.0)),
            cx,
        );
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: None,
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            display_id: Some(output.display.id()),
            window_background: WindowBackgroundAppearance::Transparent,
            app_id: Some("ghost-shell-launcher".to_owned()),
            ..Default::default()
        };

        let handle = cx.open_window(window_options, |window, cx| {
            let view = cx.new(|cx| View::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx).bordered(false))
        })?;

        self.handle = Some(handle);

        Ok(())
    }

    pub fn close(&mut self, cx: &mut App) -> Result<()> {
        self.handle.take().map_or(Ok(()), |handle| {
            handle
                .update(cx, |_view, window, _cx| window.remove_window())
                .context("failed to close launcher window")
        })
    }
}
