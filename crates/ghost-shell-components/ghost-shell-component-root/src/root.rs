// Adapted from GPUI Kit 0e63ea799766, copyright 2024 - 2026 Longbridge.
// Modified for Ghost Shell; see LICENSE.md, NOTICE, and crates/ghost-shell-components/UPSTREAM.md.

use ghost_shell_gpui::{
    AnyView, App, Context, Global, IntoElement, KeyBinding, Render, StyleRefinement,
    Styled, Subscription, Window, actions, div, prelude::*,
};
use ghost_shell_theme::{ActiveTheme as _, Theme, active_focus_trap};

actions!(ghost_root, [Tab, TabPrev]);

const CONTEXT: &str = "GhostRoot";

struct Initialized;
impl Global for Initialized {}

pub fn init(cx: &mut App) {
    ghost_shell_theme::init(cx);
    if !cx.has_global::<Initialized>() {
        cx.bind_keys([
            KeyBinding::new("tab", Tab, Some(CONTEXT)),
            KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
        ]);
        cx.set_global(Initialized);
    }
}

pub struct Root {
    style: StyleRefinement,
    view: AnyView,
    _theme_subscription: Subscription,
}

impl Root {
    pub fn new(
        view: impl Into<AnyView>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        init(cx);
        Self {
            style: StyleRefinement::default(),
            view: view.into(),
            _theme_subscription: cx.observe_global::<Theme>(|_, cx| cx.notify()),
        }
    }

    pub fn view(&self) -> &AnyView {
        &self.view
    }

    fn on_action_tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        Self::move_focus(false, window, cx);
    }

    fn on_action_tab_prev(
        &mut self,
        _: &TabPrev,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Self::move_focus(true, window, cx);
    }

    fn move_focus(backwards: bool, window: &mut Window, cx: &mut App) {
        let trap = active_focus_trap(window, cx);
        let previous = window.focused(cx);
        let advance = |window: &mut Window, cx: &mut App| {
            if backwards {
                window.focus_prev(cx);
            } else {
                window.focus_next(cx);
            }
        };

        advance(window, cx);
        let Some(trap) = trap else {
            return;
        };

        // Bound traversal if the tab order changes while handling focus events.
        for _ in 0..100 {
            if trap.contains_focused(window, cx) {
                return;
            }
            if window.focused(cx) == previous {
                break;
            }
            advance(window, cx);
        }

        if !trap.contains_focused(window, cx) {
            if let Some(previous) = previous {
                window.focus(&previous, cx);
            }
        }
    }
}

impl Styled for Root {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for Root {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        window.set_rem_size(cx.theme().tokens.typography.md.size);

        div()
            .id("ghost-root")
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_action_tab))
            .on_action(cx.listener(Self::on_action_tab_prev))
            .relative()
            .size_full()
            .font_family(cx.theme().tokens.typography.sans.clone())
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .map(|mut element| {
                element.style().refine(&self.style);
                element
            })
            .child(self.view.clone())
    }
}

#[cfg(test)]
mod tests {
    use ghost_shell_gpui::{FocusHandle, TestAppContext, px};
    use ghost_shell_theme::{FocusTrapElement as _, ThemeMode};

    use super::*;

    struct Controls {
        handles: [FocusHandle; 3],
        trap: FocusHandle,
        trapped: bool,
    }

    impl Render for Controls {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let first = div()
                .id("first")
                .track_focus(&self.handles[0])
                .child("First");
            let second = div()
                .id("second")
                .track_focus(&self.handles[1])
                .child("Second");
            let inside = div().child(first).child(second);
            div()
                .child(if self.trapped {
                    inside
                        .focus_trap("trap", &self.trap)
                        .into_any_element()
                } else {
                    inside.into_any_element()
                })
                .child(
                    div()
                        .id("outside")
                        .track_focus(&self.handles[2])
                        .child("Outside"),
                )
        }
    }

    #[ghost_shell_gpui::test]
    fn root_applies_theme_and_preserves_customization(cx: &mut TestAppContext) {
        cx.update(|cx| {
            init(cx);
            let mut theme = Theme::new(ThemeMode::Dark);
            theme.tokens.typography.md.size = px(19.);
            Theme::set(theme, cx);
        });
        let (_root, window) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| Controls {
                handles: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
                trap: cx.focus_handle(),
                trapped: false,
            });
            Root::new(view, window, cx)
        });
        window.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(window.rem_size(), px(19.));
        });
        window.update(|window, cx| {
            let mut theme = Theme::new(ThemeMode::Light);
            theme.tokens.typography.md.size = px(22.);
            Theme::set(theme, cx);
            window.draw(cx).clear(cx);
            assert_eq!(window.rem_size(), px(22.));
        });
    }

    #[ghost_shell_gpui::test]
    fn tab_navigation_without_a_trap(cx: &mut TestAppContext) {
        let (_, window) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| Controls {
                handles: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
                trap: cx.focus_handle(),
                trapped: false,
            });
            let first = view.read(cx).handles[0].clone();
            first.focus(window, cx);
            Root::new(view, window, cx)
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let first = window.update(|window, cx| window.focused(cx));
        window.simulate_keystrokes("tab");
        let second = window.update(|window, cx| window.focused(cx));
        window.simulate_keystrokes("tab");
        let third = window.update(|window, cx| window.focused(cx));
        assert_ne!(first, second);
        assert_ne!(first, third);
        assert_ne!(second, third);
        window.simulate_keystrokes("shift-tab");
        assert_eq!(window.update(|window, cx| window.focused(cx)), second);
    }

    #[ghost_shell_gpui::test]
    fn tab_navigation_and_focus_trap(cx: &mut TestAppContext) {
        let (_, window) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| Controls {
                handles: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
                trap: cx.focus_handle(),
                trapped: true,
            });
            let first = view.read(cx).handles[0].clone();
            first.focus(window, cx);
            Root::new(view, window, cx)
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let first = window.update(|window, cx| window.focused(cx));
        window.simulate_keystrokes("tab");
        let second = window.update(|window, cx| window.focused(cx));
        assert_ne!(first, second);
        window.simulate_keystrokes("tab");
        assert_eq!(window.update(|window, cx| window.focused(cx)), first);
        window.simulate_keystrokes("shift-tab");
        assert_eq!(window.update(|window, cx| window.focused(cx)), second);
    }
}
