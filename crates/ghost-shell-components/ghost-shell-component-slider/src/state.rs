// Adapted from GPUI Kit 0e63ea799766, copyright 2024 - 2026 Longbridge.
// Modified for Ghost Shell; see crates/ghost-shell-components/UPSTREAM.md.

use std::ops::Range;

use ghost_shell_gpui::{
    AccessibleAction, Along, AnyElement, App, AppContext as _, Axis, Bounds, Context,
    Div, DragMoveEvent, Empty, Entity, EntityId, EventEmitter, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, Orientation, ParentElement, Pixels, Point,
    Render, RenderOnce, Role, StatefulInteractiveElement, StyleRefinement, Styled,
    Window, div, prelude::FluentBuilder as _, px,
};

/// Events emitted by the [`SliderState`].
pub enum SliderEvent {
    /// Emitted continuously while the slider value is being changed by the user.
    Change(SliderValue),
    /// Emitted when a pointer gesture ends or a keyboard/accessibility adjustment completes.
    Release(SliderValue),
}

/// The value of the slider, can be a single value or a range of values.
///
/// - Can from a f32 value, which will be treated as a single value.
/// - Or from a (f32, f32) tuple, which will be treated as a range of values.
///
/// The default value is `SliderValue::Single(0.0)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SliderValue {
    Single(f32),
    Range(f32, f32),
}

impl std::fmt::Display for SliderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SliderValue::Single(value) => write!(f, "{}", value),
            SliderValue::Range(start, end) => write!(f, "{}..{}", start, end),
        }
    }
}

impl From<f32> for SliderValue {
    fn from(value: f32) -> Self {
        SliderValue::Single(value)
    }
}

impl From<(f32, f32)> for SliderValue {
    fn from(value: (f32, f32)) -> Self {
        SliderValue::Range(value.0, value.1)
    }
}

impl From<Range<f32>> for SliderValue {
    fn from(value: Range<f32>) -> Self {
        SliderValue::Range(value.start, value.end)
    }
}

impl Default for SliderValue {
    fn default() -> Self {
        SliderValue::Single(0.)
    }
}

impl SliderValue {
    /// Clamp and order values. NaN becomes the minimum; invalid bounds collapse to a finite point.
    pub fn clamp(self, min: f32, max: f32) -> Self {
        let min = if min.is_finite() { min } else { 0. };
        let max = if max.is_finite() { max.max(min) } else { min };
        let clamp = |value: f32| {
            if value.is_nan() {
                min
            } else {
                value.clamp(min, max)
            }
        };
        match self {
            Self::Single(value) => Self::Single(clamp(value)),
            Self::Range(start, end) => {
                let start = clamp(start);
                let end = clamp(end);
                Self::Range(start.min(end), start.max(end))
            }
        }
    }

    /// Check if the value is a single value.
    #[inline]
    pub fn is_single(&self) -> bool {
        matches!(self, SliderValue::Single(_))
    }

    /// Check if the value is a range of values.
    #[inline]
    pub fn is_range(&self) -> bool {
        matches!(self, SliderValue::Range(_, _))
    }

    /// Get the start value.
    pub fn start(&self) -> f32 {
        match self {
            SliderValue::Single(value) => *value,
            SliderValue::Range(start, _) => *start,
        }
    }

    /// Get the end value.
    pub fn end(&self) -> f32 {
        match self {
            SliderValue::Single(value) => *value,
            SliderValue::Range(_, end) => *end,
        }
    }

    fn set_start(&mut self, value: f32) {
        if let SliderValue::Range(_, end) = self {
            *self = SliderValue::Range(value.min(*end), *end);
        } else {
            *self = SliderValue::Single(value);
        }
    }

    fn set_end(&mut self, value: f32) {
        if let SliderValue::Range(start, _) = self {
            *self = SliderValue::Range(*start, value.max(*start));
        } else {
            *self = SliderValue::Single(value);
        }
    }
}

/// The scale mode of the slider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SliderScale {
    /// Linear scale where values change uniformly across the slider range.
    /// This is the default mode.
    #[default]
    Linear,
    /// Logarithmic scale where the distance between values increases exponentially.
    ///
    /// This is useful for parameters that have a large range of values where smaller
    /// changes are more significant at lower values. Common examples include:
    ///
    /// - Volume controls (human hearing perception is logarithmic)
    /// - Frequency controls (musical notes follow a logarithmic scale)
    /// - Zoom levels
    /// - Any parameter where you want finer control at lower values
    ///
    /// # For example
    ///
    /// ```
    /// use ghost_shell_component_slider::{SliderScale, SliderState};
    ///
    /// let slider = SliderState::new()
    ///     .min(1.0)    // Must be > 0 for logarithmic scale
    ///     .max(1000.0)
    ///     .scale(SliderScale::Logarithmic);
    /// ```
    ///
    /// - Moving the slider 1/3 of the way will yield ~10
    /// - Moving it 2/3 of the way will yield ~100
    /// - The full range covers 3 orders of magnitude evenly
    Logarithmic,
}

impl SliderScale {
    #[inline]
    pub fn is_linear(&self) -> bool {
        matches!(self, SliderScale::Linear)
    }

    #[inline]
    pub fn is_logarithmic(&self) -> bool {
        matches!(self, SliderScale::Logarithmic)
    }
}

/// State of the [`crate::Slider`].
pub struct SliderState {
    min: f32,
    max: f32,
    step: f32,
    value: SliderValue,
    /// When is single value mode, only `end` is used, the start is always 0.0.
    percentage: Range<f32>,
    /// The bounds of the slider after rendered.
    bounds: Bounds<Pixels>,
    scale: SliderScale,
    /// Tracks whether the user is currently interacting with the slider so we
    /// only emit [`SliderEvent::Release`] after a real press/drag.
    dragging: bool,
}

impl Default for SliderState {
    fn default() -> Self {
        Self::new()
    }
}

impl SliderState {
    /// Create a new [`SliderState`].
    pub fn new() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            value: SliderValue::default(),
            percentage: (0.0..0.0),
            bounds: Bounds::default(),
            scale: SliderScale::default(),
            dragging: false,
        }
    }

    /// Set the minimum, extending the maximum if needed. Non-finite inputs are ignored.
    pub fn min(mut self, min: f32) -> Self {
        if min.is_finite() {
            self.min = min;
            self.max = self.max.max(min);
            self.update_thumb_pos();
        }
        self
    }

    /// Set the maximum, extending the minimum if needed. Non-finite inputs are ignored.
    pub fn max(mut self, max: f32) -> Self {
        if max.is_finite() {
            self.max = max;
            self.min = self.min.min(max);
            self.update_thumb_pos();
        }
        self
    }

    /// Set a positive, finite step. Invalid inputs retain the previous step.
    pub fn step(mut self, step: f32) -> Self {
        if step.is_finite() && step > 0. {
            self.step = step;
        }
        self
    }

    /// Set the scale. Logarithmic mapping falls back to linear unless 0 < min < max.
    pub fn scale(mut self, scale: SliderScale) -> Self {
        self.scale = scale;
        self.update_thumb_pos();
        self
    }

    /// Set the default value of the slider, default: 0.0
    pub fn default_value(mut self, value: impl Into<SliderValue>) -> Self {
        self.value = value.into();
        self.update_thumb_pos();
        self
    }

    /// Set a clamped value and notify observers without emitting user interaction events.
    pub fn set_value(
        &mut self,
        value: impl Into<SliderValue>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.value = value.into();
        self.update_thumb_pos();
        cx.notify();
    }

    /// Get the value of the slider.
    pub fn value(&self) -> SliderValue {
        self.value
    }

    /// Get the minimum value.
    pub fn min_value(&self) -> f32 {
        self.min
    }

    /// Get the maximum value.
    pub fn max_value(&self) -> f32 {
        self.max
    }

    /// Get the step value.
    pub fn step_value(&self) -> f32 {
        self.step
    }

    /// Converts a value between 0.0 and 1.0 to a value between the minimum and maximum value,
    /// depending on the chosen scale.
    fn percentage_to_value(&self, percentage: f32) -> f32 {
        let min = f64::from(self.min);
        let max = f64::from(self.max);
        let percentage = f64::from(percentage.clamp(0., 1.));
        if self.scale.is_logarithmic() && min > 0. && max > min {
            (min.ln() + (max.ln() - min.ln()) * percentage).exp() as f32
        } else {
            (min + (max - min) * percentage) as f32
        }
    }

    fn value_to_percentage(&self, value: f32) -> f32 {
        let min = f64::from(self.min);
        let max = f64::from(self.max);
        if max <= min {
            return 0.;
        }
        let value = f64::from(value);
        let percentage = if self.scale.is_logarithmic() && min > 0. {
            (value.ln() - min.ln()) / (max.ln() - min.ln())
        } else {
            (value - min) / (max - min)
        };
        percentage.clamp(0., 1.) as f32
    }

    fn update_thumb_pos(&mut self) {
        self.value = self.value.clamp(self.min, self.max);
        match self.value {
            SliderValue::Single(value) => {
                let percentage =
                    self.value_to_percentage(value.clamp(self.min, self.max));
                self.percentage = 0.0..percentage;
            }
            SliderValue::Range(start, end) => {
                let clamped_start = start.clamp(self.min, self.max);
                let clamped_end = end.clamp(self.min, self.max);
                self.percentage = self.value_to_percentage(clamped_start)
                    ..self.value_to_percentage(clamped_end);
            }
        }
    }

    /// Update value by mouse position
    pub(crate) fn update_value_by_position(
        &mut self,
        axis: Axis,
        position: Point<Pixels>,
        is_start: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let bounds = self.bounds;

        let inner_pos = if axis == Axis::Horizontal {
            position.x - bounds.left()
        } else {
            bounds.bottom() - position.y
        };
        let total_size = bounds.size.along(axis);
        if total_size <= px(0.) || !f32::from(total_size).is_finite() {
            return;
        }
        let percentage = inner_pos.clamp(px(0.), total_size) / total_size;
        if !percentage.is_finite() {
            return;
        }
        self.dragging = true;

        let percentage = if is_start {
            percentage.clamp(0.0, self.percentage.end)
        } else {
            percentage.clamp(self.percentage.start, 1.0)
        };

        let value = self.percentage_to_value(percentage);
        let value = if percentage <= 0. {
            self.min
        } else if percentage >= 1. {
            self.max
        } else {
            let min = f64::from(self.min);
            let step = f64::from(self.step);
            (min + ((f64::from(value) - min) / step).round() * step) as f32
        }
        .clamp(self.min, self.max);
        self.change_value(value, is_start, cx);
    }

    fn change_value(&mut self, value: f32, start: bool, cx: &mut Context<Self>) {
        let previous = self.value;
        if start {
            self.value.set_start(value);
        } else {
            self.value.set_end(value);
        }
        self.update_thumb_pos();
        if previous != self.value {
            cx.emit(SliderEvent::Change(self.value));
            cx.notify();
        }
    }

    /// Emit [`SliderEvent::Release`] if the user was actively interacting
    /// with the slider. Called on mouse-up both inside and outside the slider.
    pub(crate) fn handle_release(&mut self, cx: &mut Context<Self>) {
        if !self.dragging {
            return;
        }
        self.dragging = false;
        cx.emit(SliderEvent::Release(self.value));
    }
}

#[derive(Clone)]
struct DragThumb((EntityId, bool));

impl Render for DragThumb {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone)]
struct DragSlider(EntityId);

impl Render for DragSlider {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// An unstyled slider behavior root.
///
/// Applications provide the track, range, and thumb presentation as children.
#[derive(IntoElement)]
pub(crate) struct BaseSlider {
    state: Entity<SliderState>,
    axis: Axis,
    disabled: bool,
    base: Div,
    children: Vec<AnyElement>,
}

impl BaseSlider {
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            state: state.clone(),
            axis: Axis::Horizontal,
            disabled: false,
            base: div(),
            children: Vec::new(),
        }
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ParentElement for BaseSlider {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for BaseSlider {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for BaseSlider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let axis = self.axis;
        let entity_id = self.state.entity_id();
        let state = self.state.read(cx);
        let slider_state = self.state.clone();

        self.base
            .id(("slider", entity_id))
            .tab_group()
            .role(Role::Slider)
            .aria_numeric_value(state.value().end() as f64)
            .aria_min_numeric_value(state.min_value() as f64)
            .aria_max_numeric_value(state.max_value() as f64)
            .aria_numeric_value_step(state.step_value() as f64)
            .aria_orientation(if axis == Axis::Vertical {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            })
            .when(!self.disabled, |this| {
                this.on_a11y_action(AccessibleAction::Increment, {
                    let state = slider_state.clone();
                    move |_, _, cx| {
                        state.update(cx, |state, cx| {
                            let value = (state.value().end() + state.step_value())
                                .min(state.max_value());
                            state.change_value(value, false, cx);
                            cx.emit(SliderEvent::Release(state.value));
                        });
                    }
                })
                .on_a11y_action(AccessibleAction::Decrement, {
                    let state = slider_state.clone();
                    move |_, _, cx| {
                        state.update(cx, |state, cx| {
                            let value = (state.value().end() - state.step_value())
                                .max(state.min_value());
                            state.change_value(value, false, cx);
                            cx.emit(SliderEvent::Release(state.value));
                        });
                    }
                })
            })
            .when(!self.disabled, |this| {
                this.on_mouse_up(
                    MouseButton::Left,
                    window.listener_for(&self.state, |state, _, _, cx| {
                        state.handle_release(cx)
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    window.listener_for(&self.state, |state, _, _, cx| {
                        state.handle_release(cx)
                    }),
                )
            })
            .children(self.children)
    }
}

/// An unstyled track that records the geometry used to map pointer positions.
#[derive(IntoElement)]
pub struct SliderTrack {
    state: Entity<SliderState>,
    axis: Axis,
    disabled: bool,
    base: Div,
    children: Vec<AnyElement>,
}

impl SliderTrack {
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            state: state.clone(),
            axis: Axis::Horizontal,
            disabled: false,
            base: div(),
            children: Vec::new(),
        }
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ParentElement for SliderTrack {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for SliderTrack {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for SliderTrack {
    fn interactivity(&mut self) -> &mut ghost_shell_gpui::Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for SliderTrack {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let axis = self.axis;
        let entity_id = self.state.entity_id();
        let state = self.state.read(cx);
        let is_range = state.value().is_range();
        let percentage = state.percentage();
        self.base
            .id("slider-bar-container")
            .children(self.children)
            .when(!self.disabled, |this| {
                this.on_mouse_down(
                    MouseButton::Left,
                    window.listener_for(
                        &self.state,
                        move |state, event: &MouseDownEvent, window, cx| {
                            let is_start = if is_range {
                                let size = state.bounds().size.along(axis);
                                let position = if axis == Axis::Horizontal {
                                    event.position.x - state.bounds().left()
                                } else {
                                    state.bounds().bottom() - event.position.y
                                };
                                let center = ((percentage.end - percentage.start) / 2.
                                    + percentage.start)
                                    * size;
                                position < center
                            } else {
                                false
                            };
                            state.update_value_by_position(
                                axis,
                                event.position,
                                is_start,
                                window,
                                cx,
                            );
                        },
                    ),
                )
                .when(!is_range, |this| {
                    this.on_drag(DragSlider(entity_id), |drag, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| drag.clone())
                    })
                    .on_drag_move(window.listener_for(
                        &self.state,
                        move |state, event: &DragMoveEvent<DragSlider>, window, cx| {
                            let DragSlider(id) = event.drag(cx);
                            if *id == entity_id {
                                state.update_value_by_position(
                                    axis,
                                    event.event.position,
                                    false,
                                    window,
                                    cx,
                                );
                            }
                        },
                    ))
                })
            })
    }
}

/// An unstyled slider indicator that records the value-mapping bounds.
#[derive(IntoElement)]
pub struct SliderIndicator {
    state: Entity<SliderState>,
    base: Div,
    children: Vec<AnyElement>,
}

impl SliderIndicator {
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            state: state.clone(),
            base: div(),
            children: Vec::new(),
        }
    }
}

impl ParentElement for SliderIndicator {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for SliderIndicator {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for SliderIndicator {
    fn interactivity(&mut self) -> &mut ghost_shell_gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for SliderIndicator {}

impl RenderOnce for SliderIndicator {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .id("slider-bar")
            .children(self.children)
            .child(
                ghost_shell_gpui::canvas(
                    {
                        let state = self.state;
                        move |bounds, _, cx| {
                            state.update(cx, |state, _| state.set_bounds(bounds))
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
    }
}

/// An unstyled draggable slider thumb.
#[derive(IntoElement)]
pub struct SliderThumb {
    state: Entity<SliderState>,
    axis: Axis,
    start: bool,
    disabled: bool,
    base: Div,
    children: Vec<AnyElement>,
}

impl SliderThumb {
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            state: state.clone(),
            axis: Axis::Horizontal,
            start: false,
            disabled: false,
            base: div(),
            children: Vec::new(),
        }
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    pub fn start(mut self, start: bool) -> Self {
        self.start = start;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ParentElement for SliderThumb {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for SliderThumb {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for SliderThumb {
    fn interactivity(&mut self) -> &mut ghost_shell_gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for SliderThumb {}

impl RenderOnce for SliderThumb {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        let entity_id = self.state.entity_id();
        let axis = self.axis;
        let start = self.start;
        self.base
            .id(("slider-thumb", start as u32))
            .children(self.children)
            .when(!self.disabled, |this| {
                this.tab_index(if start { 0 } else { 1 })
                    .on_key_down(window.listener_for(
                        &self.state,
                        move |state, event: &ghost_shell_gpui::KeyDownEvent, _, cx| {
                            let current = if start {
                                state.value.start()
                            } else {
                                state.value.end()
                            };
                            let value = match event.keystroke.key.as_str() {
                                "left" | "down" => current - state.step,
                                "right" | "up" => current + state.step,
                                "home" => state.min,
                                "end" => state.max,
                                _ => return,
                            };
                            state.change_value(
                                value.clamp(state.min, state.max),
                                start,
                                cx,
                            );
                            cx.emit(SliderEvent::Release(state.value));
                            cx.stop_propagation();
                        },
                    ))
                    .on_mouse_down(
                        MouseButton::Left,
                        window.listener_for(&self.state, |state, _, _, cx| {
                            state.dragging = true;
                            cx.stop_propagation();
                        }),
                    )
                    .on_drag(DragThumb((entity_id, start)), |drag, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| drag.clone())
                    })
                    .on_drag_move(window.listener_for(
                        &self.state,
                        move |state, event: &DragMoveEvent<DragThumb>, window, cx| {
                            let DragThumb((id, start)) = event.drag(cx);
                            if *id == entity_id {
                                state.update_value_by_position(
                                    axis,
                                    event.event.position,
                                    *start,
                                    window,
                                    cx,
                                );
                            }
                        },
                    ))
            })
    }
}

impl EventEmitter<SliderEvent> for SliderState {}

impl SliderState {
    pub(crate) fn percentage(&self) -> Range<f32> {
        self.percentage.clone()
    }

    pub(crate) fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    pub(crate) fn set_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = bounds;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_value_conversions_and_clamping_are_preserved() {
        assert_eq!(SliderValue::from(5.), SliderValue::Single(5.));
        assert_eq!(SliderValue::from((2., 8.)), SliderValue::Range(2., 8.));
        assert_eq!(SliderValue::from(2.0..8.0), SliderValue::Range(2., 8.));
        assert_eq!(
            SliderValue::Range(-1., 12.).clamp(0., 10.),
            SliderValue::Range(0., 10.)
        );
    }

    #[test]
    fn legacy_linear_state_keeps_percentage_and_range_ordering() {
        let state = SliderState::new()
            .min(0.)
            .max(200.)
            .default_value((50., 150.));
        assert_eq!(state.value(), SliderValue::Range(50., 150.));
        assert_eq!(state.percentage(), 0.25..0.75);
    }

    #[test]
    fn legacy_logarithmic_state_keeps_mapping() {
        let state = SliderState::new()
            .min(1.)
            .max(1000.)
            .scale(SliderScale::Logarithmic)
            .default_value(10.);
        let percentage = state.percentage().end;
        assert!((percentage - (1. / 3.)).abs() < 0.0001);
    }

    #[test]
    fn values_are_normalized_and_extreme_mappings_remain_finite() {
        let state = SliderState::new().default_value((120., -20.));
        assert_eq!(state.value(), SliderValue::Range(0., 100.));
        let state = SliderState::new()
            .min(-f32::MAX)
            .max(f32::MAX)
            .default_value(0.);
        assert_eq!(state.percentage(), 0.0..0.5);
        assert_eq!(state.percentage_to_value(0.5), 0.);
        let state = SliderState::new()
            .min(f32::MIN_POSITIVE)
            .max(f32::MAX)
            .scale(SliderScale::Logarithmic)
            .default_value(1.);
        assert!(state.percentage().end.is_finite());
        assert!(state.percentage_to_value(0.5).is_finite());
    }

    #[test]
    fn invalid_configuration_stays_finite() {
        let state = SliderState::new()
            .scale(SliderScale::Logarithmic)
            .step(0.)
            .min(200.)
            .max(100.)
            .default_value(f32::NAN);
        assert_eq!(state.value(), SliderValue::Single(100.));
        assert_eq!(state.percentage(), 0.0..0.0);
        assert_eq!(state.step_value(), 1.);
    }
}
