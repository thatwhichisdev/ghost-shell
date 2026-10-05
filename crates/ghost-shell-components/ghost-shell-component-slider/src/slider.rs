// Adapted from GPUI Kit 0e63ea799766, copyright 2024 - 2026 Longbridge.
// Modified for Ghost Shell; see crates/ghost-shell-components/UPSTREAM.md.

mod state;

use ghost_shell_gpui::{
    App, Axis, Background, Corners, DefiniteLength, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, Refineable as _, RenderOnce,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, px, relative,
};
use ghost_shell_theme::ActiveTheme as _;
use state::{BaseSlider, SliderIndicator, SliderThumb, SliderTrack};
pub use state::{SliderEvent, SliderScale, SliderState, SliderValue};

/// A Slider element.
#[derive(IntoElement)]
pub struct Slider {
    state: Entity<SliderState>,
    axis: Axis,
    style: StyleRefinement,
    disabled: bool,
    reverse: bool,
}

impl Slider {
    /// Create a new [`Slider`] element bind to the [`SliderState`].
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            axis: Axis::Horizontal,
            state: state.clone(),
            style: StyleRefinement::default(),
            disabled: false,
            reverse: false,
        }
    }

    /// As a horizontal slider.
    pub fn horizontal(mut self) -> Self {
        self.axis = Axis::Horizontal;
        self
    }

    /// As a vertical slider.
    pub fn vertical(mut self) -> Self {
        self.axis = Axis::Vertical;
        self
    }

    /// Set the disabled state of the slider, default: false
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Reverse the filled (highlighted) side of the track, default: false.
    ///
    /// By default the track is filled from the min end to the thumb. With
    /// `reverse`, the fill goes from the thumb to the max end instead — useful
    /// when the slider represents a remaining amount (e.g. time left).
    ///
    /// This only changes the visual fill; values, events and interactions are
    /// unaffected. It applies to single-value sliders and is ignored for
    /// range sliders.
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }
}

impl Styled for Slider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let axis = self.axis;
        let state = self.state.read(cx);
        let is_range = state.value().is_range();
        let percentage = state.percentage();
        let (bar_start, bar_end) = if self.reverse && !is_range {
            // Fill from the thumb to the max end (remaining side).
            (relative(percentage.end), relative(0.))
        } else {
            (relative(percentage.start), relative(1. - percentage.end))
        };
        let rem_size = window.rem_size();

        let bar_color = self
            .style
            .background
            .clone()
            .and_then(|bg| bg.color())
            .unwrap_or(cx.theme().primary.into());
        let thumb_bg: Background = self
            .style
            .text
            .color
            .map(Into::into)
            .unwrap_or_else(|| cx.theme().background.into());
        let corner_radii = self.style.corner_radii.clone();
        // The track is a pill by default, and square when the theme squares its
        // corners. A caller's own corner radii still win.
        let default_radius = cx.theme().tokens.radius.full;
        let radius = Corners {
            top_left: corner_radii
                .top_left
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
            top_right: corner_radii
                .top_right
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
            bottom_left: corner_radii
                .bottom_left
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
            bottom_right: corner_radii
                .bottom_right
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
        };

        let ring_color = cx.theme().ring;
        let thumb = |position: DefiniteLength, start: bool| {
            SliderThumb::new(&self.state)
                .axis(axis)
                .start(start)
                .disabled(self.disabled)
                .when(!self.disabled, |this| {
                    this.absolute()
                        .when(axis == Axis::Horizontal, |this| {
                            this.top(px(-5.)).left(position).ml(-px(8.))
                        })
                        .when(axis == Axis::Vertical, |this| {
                            this.bottom(position)
                                .left(px(-5.))
                                .mb(-px(8.))
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .flex_shrink_0()
                        .rounded_full()
                        .bg(bar_color.opacity(0.5))
                        .hover(move |style| style.bg(ring_color))
                        .focus(move |style| style.bg(ring_color))
                        .size_4()
                        .p(px(1.))
                        .child(
                            div()
                                .flex_shrink_0()
                                .size_full()
                                .rounded_full()
                                .bg(thumb_bg),
                        )
                })
        };

        BaseSlider::new(&self.state)
            .axis(axis)
            .disabled(self.disabled)
            .flex()
            .flex_1()
            .items_center()
            .justify_center()
            .when(axis == Axis::Vertical, |this| this.h(px(120.)))
            .when(axis == Axis::Horizontal, |this| this.w_full())
            .map(|mut element| {
                element.style().refine(&self.style);
                element
            })
            .bg(ghost_shell_gpui::transparent_black())
            .text_color(cx.theme().foreground)
            .child(
                SliderTrack::new(&self.state)
                    .axis(axis)
                    .disabled(self.disabled)
                    .flex()
                    .when(axis == Axis::Horizontal, |this| {
                        this.items_center().h_6().w_full()
                    })
                    .when(axis == Axis::Vertical, |this| {
                        this.justify_center().w_6().h_full()
                    })
                    .flex_shrink_0()
                    .child(
                        SliderIndicator::new(&self.state)
                            .relative()
                            .when(axis == Axis::Horizontal, |this| this.w_full().h_1p5())
                            .when(axis == Axis::Vertical, |this| this.h_full().w_1p5())
                            .bg(bar_color.opacity(0.2))
                            .active(|this| this.bg(bar_color.opacity(0.4)))
                            .rounded_tl(radius.top_left)
                            .rounded_tr(radius.top_right)
                            .rounded_bl(radius.bottom_left)
                            .rounded_br(radius.bottom_right)
                            .child(
                                div()
                                    .absolute()
                                    .when(axis == Axis::Horizontal, |this| {
                                        this.h_full().left(bar_start).right(bar_end)
                                    })
                                    .when(axis == Axis::Vertical, |this| {
                                        this.w_full().bottom(bar_start).top(bar_end)
                                    })
                                    .bg(bar_color)
                                    .rounded_full(),
                            )
                            .when(is_range, |this| {
                                this.child(thumb(relative(percentage.start), true))
                            })
                            .child(thumb(relative(percentage.end), false)),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use ghost_shell_gpui::{
        AppContext as _, Context, Modifiers, Render, TestAppContext, point,
    };

    use super::*;

    struct Harness {
        state: Entity<SliderState>,
        disabled: bool,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(100.))
                .h(px(24.))
                .child(Slider::new(&self.state).disabled(self.disabled))
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        disabled: bool,
    ) -> (
        &mut ghost_shell_gpui::VisualTestContext,
        Entity<SliderState>,
    ) {
        cx.update(ghost_shell_theme::init);
        let state = cx.new(|_| SliderState::new());
        let result = state.clone();
        let (_, cx) = cx.add_window_view(move |_, _| Harness { state, disabled });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (cx, result)
    }

    #[ghost_shell_gpui::test]
    fn pointer_updates_the_migrated_state(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, false);
        cx.simulate_click(point(px(50.), px(12.)), Modifiers::default());
        cx.update(|_, cx| assert!((state.read(cx).value().end() - 50.).abs() < 1.));
    }

    #[ghost_shell_gpui::test]
    fn disabled_slider_is_inert(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, true);
        cx.simulate_click(point(px(50.), px(12.)), Modifiers::default());
        cx.update(|_, cx| assert_eq!(state.read(cx).value(), SliderValue::Single(0.)));
    }
}

#[cfg(test)]
mod interaction_tests {
    use std::{cell::RefCell, rc::Rc};

    use ghost_shell_gpui::{
        AppContext as _, Context, Modifiers, MouseButton, Render, TestAppContext, point,
    };

    use super::*;

    struct Harness {
        state: Entity<SliderState>,
        vertical: bool,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(100.)).h(px(100.)).child(
                Slider::new(&self.state)
                    .when(self.vertical, |slider| slider.vertical().h(px(100.))),
            )
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        state: SliderState,
        vertical: bool,
    ) -> (
        &mut ghost_shell_gpui::VisualTestContext,
        Entity<SliderState>,
    ) {
        cx.update(ghost_shell_theme::init);
        let state = cx.new(|_| state);
        let result = state.clone();
        let (_, cx) = cx.add_window_view(move |_, _| Harness { state, vertical });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (cx, result)
    }

    #[ghost_shell_gpui::test]
    fn snapping_respects_offset_and_endpoints(cx: &mut TestAppContext) {
        let (cx, state) =
            harness(cx, SliderState::new().min(5.).max(14.).step(4.), false);
        cx.simulate_click(point(px(50.), px(12.)), Modifiers::default());
        cx.update(|_, cx| {
            assert_eq!(state.read(cx).value(), SliderValue::Single(9.));
            assert!((state.read(cx).percentage().end - 4. / 9.).abs() < 0.0001);
        });
        cx.simulate_click(point(px(99.), px(12.)), Modifiers::default());
        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.update_value_by_position(
                    Axis::Horizontal,
                    point(px(200.), px(12.)),
                    false,
                    window,
                    cx,
                );
                assert_eq!(state.value(), SliderValue::Single(14.));
            });
        });
    }

    #[ghost_shell_gpui::test]
    fn vertical_and_range_mapping(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, SliderState::new().default_value((20., 80.)), true);
        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                let bounds = state.bounds();
                state.update_value_by_position(
                    Axis::Vertical,
                    point(bounds.left(), bounds.top()),
                    true,
                    window,
                    cx,
                );
                assert_eq!(state.value(), SliderValue::Range(80., 80.));
                state.update_value_by_position(
                    Axis::Vertical,
                    point(bounds.left(), bounds.bottom()),
                    false,
                    window,
                    cx,
                );
                assert_eq!(state.value(), SliderValue::Range(80., 80.));
            });
        });
    }

    #[ghost_shell_gpui::test]
    fn drag_release_outside_fires_once_and_programmatic_updates_are_silent(
        cx: &mut TestAppContext,
    ) {
        let (cx, state) = harness(cx, SliderState::new().default_value(50.), false);
        let events = Rc::new(RefCell::new(Vec::new()));
        let subscription = cx.update(|_, cx| {
            let events = events.clone();
            cx.subscribe(&state, move |_, event, _| {
                events.borrow_mut().push(match event {
                    SliderEvent::Change(value) => (false, *value),
                    SliderEvent::Release(value) => (true, *value),
                });
            })
        });
        cx.simulate_mouse_down(
            point(px(50.), px(12.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_move(
            point(px(75.), px(12.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_move(
            point(px(150.), px(12.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(150.), px(12.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(150.), px(12.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| event.0)
                .count(),
            1
        );
        assert!(events.borrow().iter().any(|event| !event.0));
        events.borrow_mut().clear();
        cx.update(|window, cx| {
            state.update(cx, |state, cx| state.set_value(30., window, cx))
        });
        assert!(events.borrow().is_empty());
        drop(subscription);
    }

    #[ghost_shell_gpui::test]
    fn thumbs_support_keyboard_adjustments(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, SliderState::new().default_value(50.), false);
        cx.update(|window, cx| window.focus_next(cx));
        cx.simulate_keystrokes("right");
        cx.update(|_, cx| assert_eq!(state.read(cx).value(), SliderValue::Single(51.)));
        cx.simulate_keystrokes("home");
        cx.update(|_, cx| assert_eq!(state.read(cx).value(), SliderValue::Single(0.)));
        cx.simulate_keystrokes("end");
        cx.update(|_, cx| assert_eq!(state.read(cx).value(), SliderValue::Single(100.)));
    }
}
