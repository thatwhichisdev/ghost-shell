use anyhow::Result;
use ghost_shell_config::AppConfig;
use gpui::{
    App, DisplayId, Global, KeyBinding, Pixels, PlatformDisplay, Size,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
    actions,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    point,
    prelude::*,
    px,
};
use std::rc::Rc;

pub struct BarConfig {
    pub display_id: DisplayId,
    pub size: Size<Pixels>,
}

pub struct AppState {
    config: AppConfig,
    bars: Vec<BarConfig>,
}

impl AppState {
    #[must_use]
    pub fn new(
        config: AppConfig,
        displays: &[Rc<dyn PlatformDisplay>],
    ) -> Self {
        let bars: Vec<BarConfig> = displays
            .iter()
            .map(|display| {
                let display_size = display.bounds().size;
                BarConfig {
                    display_id: display.id(),
                    size: Size::new(display_size.width, px(config.bar_height)),
                }
            })
            .collect();

        Self { config, bars }
    }
}

impl Global for AppState {}

actions!(window, [Quit]);

/// Initializes shell
///
/// # Errors
/// Bubbles up errors from gpui
pub fn init(cx: &mut App) -> Result<()> {
    let app_config = match ghost_shell_config::load() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Failed to load config, using default {err:?}");
            AppConfig::default()
        }
    };

    let app_state = AppState::new(app_config.clone(), &cx.displays());

    let windows_options: Vec<WindowOptions> = app_state
        .bars
        .iter()
        .map(|bar| {
            let app_id: String = format!("ghost-shell-{:?}", bar.display_id);
            let namespace: String = format!("namespace-{:?}", bar.display_id);
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(gpui::Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: bar.size,
                })),
                titlebar: None,
                focus: false,
                show: true,
                kind: WindowKind::LayerShell(LayerShellOptions {
                    namespace,
                    layer: Layer::Top,
                    anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
                    exclusive_zone: Some(px(app_state
                        .config
                        .bar_exclusive_zone)),
                    keyboard_interactivity: KeyboardInteractivity::OnDemand,
                    ..Default::default()
                }),
                is_movable: true,
                app_owns_titlebar_drag: false,
                is_resizable: true,
                is_minimizable: true,
                display_id: Some(bar.display_id),
                window_background: WindowBackgroundAppearance::Blurred,
                app_id: Some(app_id),
                window_min_size: None,
                window_decorations: None,
                icon: None,
                tabbing_identifier: None,
            }
        })
        .collect();

    for window_options in windows_options {
        cx.open_window(window_options, |_, cx| {
            cx.new(|_| ghost_shell_bar::Bar)
        })?;
    }

    cx.set_global(app_config);
    cx.set_global(app_state);

    cx.on_action(|_: &Quit, cx| cx.quit());

    cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

    Ok(())
}
