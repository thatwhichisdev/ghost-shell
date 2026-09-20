// Adapted from GPUI Kit's single-line input, selection and IME behavior.
// Copyright 2024 - 2026 Longbridge; see ../UPSTREAM.md in the components directory.

use std::{collections::VecDeque, ops::Range, time::Duration};

use ghost_shell_gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, EntityInputHandler, EventEmitter,
    FocusHandle, Focusable, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, Render, Role, ShapedLine, SharedString, Subscription,
    Task, UTF16Selection, Window, div, point, prelude::*, px,
};
use unicode_segmentation::UnicodeSegmentation;

use super::{element::TextElement, *};

#[derive(Clone)]
struct EditSnapshot {
    value: SharedString,
    selection: Range<usize>,
    reversed: bool,
}

pub struct InputState {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) value: SharedString,
    pub(crate) placeholder: SharedString,
    pub(crate) selection: Range<usize>,
    reversed: bool,
    pub(crate) marked: Option<Range<usize>>,
    pub(crate) masked: bool,
    content_type: InputContentType,
    pub(crate) disabled: bool,
    selecting: bool,
    focused: bool,
    pub(crate) cursor_visible: bool,
    blink_task: Option<Task<()>>,
    undo: VecDeque<EditSnapshot>,
    redo: Vec<EditSnapshot>,
    composition: Option<EditSnapshot>,
    pub(crate) layout: Option<ShapedLine>,
    pub(crate) bounds: Option<Bounds<Pixels>>,
    pub(crate) text_origin: Point<Pixels>,
    pub(crate) scroll_offset: Pixels,
    _subscriptions: Vec<Subscription>,
}

impl InputState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        init(cx);
        let focus_handle = cx.focus_handle().tab_stop(true);
        let subscriptions = vec![
            cx.on_focus(&focus_handle, window, |state, _, cx| {
                state.focused = true;
                state.pause_blink(cx);
                cx.emit(InputEvent::Focus);
            }),
            cx.on_blur(&focus_handle, window, |state, _, cx| {
                state.focused = false;
                state.selecting = false;
                state.blink_task = None;
                state.cursor_visible = false;
                state.finish_composition();
                cx.emit(InputEvent::Blur);
                cx.notify();
            }),
        ];
        Self {
            focus_handle,
            value: "".into(),
            placeholder: "".into(),
            selection: 0..0,
            reversed: false,
            marked: None,
            masked: false,
            content_type: InputContentType::Text,
            disabled: false,
            selecting: false,
            focused: false,
            cursor_visible: false,
            blink_task: None,
            undo: VecDeque::new(),
            redo: Vec::new(),
            composition: None,
            layout: None,
            bounds: None,
            text_origin: Point::default(),
            scroll_offset: px(0.),
            _subscriptions: subscriptions,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        if masked {
            self.clear_history();
        }
        self
    }

    pub fn value(&self) -> SharedString {
        self.value.clone()
    }

    pub fn set_value(
        &mut self,
        value: impl Into<SharedString>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = normalize(&value.into());
        if self.value.as_ref() == value {
            return;
        }
        self.value = value.into();
        self.selection = self.value.len()..self.value.len();
        self.reversed = false;
        self.marked = None;
        self.clear_history();
        self.changed(cx);
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        if !self.disabled {
            self.focus_handle.focus(window, cx);
        }
    }

    pub(crate) fn configure(
        &mut self,
        disabled: bool,
        content_type: InputContentType,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.content_type != content_type {
            self.content_type = content_type;
            if self.is_secret() {
                self.clear_history();
            }
        }
        if self.disabled != disabled {
            self.disabled = disabled;
            self.focus_handle = self.focus_handle.clone().tab_stop(!disabled);
            if disabled {
                self.blink_task = None;
                self.selecting = false;
                self.cursor_visible = false;
                if self.focus_handle.is_focused(window) {
                    window.blur(cx);
                }
            } else if self.focused {
                self.pause_blink(cx);
            }
        }
    }

    pub(crate) fn is_secret(&self) -> bool {
        self.masked
            || matches!(
                self.content_type,
                InputContentType::Password | InputContentType::NewPassword
            )
    }

    fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.composition = None;
    }

    fn snapshot(&self) -> EditSnapshot {
        EditSnapshot {
            value: self.value.clone(),
            selection: self.selection.clone(),
            reversed: self.reversed,
        }
    }

    fn push_undo(&mut self, snapshot: EditSnapshot) {
        // Passwords must not linger in edit history after the visible value is cleared.
        if self.is_secret() {
            return;
        }
        if self.undo.len() >= 100 {
            self.undo.pop_front();
        }
        self.undo.push_back(snapshot);
        self.redo.clear();
    }

    fn finish_composition(&mut self) {
        self.marked = None;
        if let Some(snapshot) = self.composition.take() {
            if snapshot.value != self.value {
                self.push_undo(snapshot);
            }
        }
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        self.layout = None;
        self.pause_blink(cx);
        cx.emit(InputEvent::Change);
        cx.notify();
    }

    fn pause_blink(&mut self, cx: &mut Context<Self>) {
        self.blink_task = None;
        self.cursor_visible = self.focused && !self.disabled;
        if !self.cursor_visible {
            return;
        }
        self.blink_task = Some(cx.spawn(async move |state, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                let Some(state) = state.upgrade() else {
                    break;
                };
                state.update(cx, |state, cx| {
                    state.cursor_visible = !state.cursor_visible;
                    cx.notify();
                });
            }
        }));
    }

    pub(crate) fn cursor(&self) -> usize {
        if self.reversed {
            self.selection.start
        } else {
            self.selection.end
        }
    }

    fn move_to(&mut self, offset: usize, select: bool, cx: &mut Context<Self>) {
        self.finish_composition();
        let offset = floor_boundary(&self.value, offset);
        let anchor = if !select {
            offset
        } else if self.reversed {
            self.selection.end
        } else {
            self.selection.start
        };
        self.selection = anchor.min(offset)..anchor.max(offset);
        self.reversed = offset < anchor;
        self.pause_blink(cx);
        cx.notify();
    }

    fn previous_boundary(&self) -> usize {
        self.value
            .grapheme_indices(true)
            .rev()
            .find_map(|(index, _)| (index < self.cursor()).then_some(index))
            .unwrap_or(0)
    }

    fn next_boundary(&self) -> usize {
        self.value
            .grapheme_indices(true)
            .find_map(|(index, _)| (index > self.cursor()).then_some(index))
            .unwrap_or(self.value.len())
    }

    fn left(&mut self, _: &MoveLeft, _: &mut Window, cx: &mut Context<Self>) {
        let offset = if self.selection.is_empty() {
            self.previous_boundary()
        } else {
            self.selection.start
        };
        self.move_to(offset, false, cx);
    }
    fn right(&mut self, _: &MoveRight, _: &mut Window, cx: &mut Context<Self>) {
        let offset = if self.selection.is_empty() {
            self.next_boundary()
        } else {
            self.selection.end
        };
        self.move_to(offset, false, cx);
    }
    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.previous_boundary(), true, cx);
    }
    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.next_boundary(), true, cx);
    }
    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, false, cx);
    }
    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.value.len(), false, cx);
    }
    fn select_home(&mut self, _: &SelectHome, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, true, cx);
    }
    fn select_end(&mut self, _: &SelectEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.value.len(), true, cx);
    }
    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, false, cx);
        self.move_to(self.value.len(), true, cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        if self.selection.is_empty() {
            self.move_to(self.previous_boundary(), true, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }
    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        if self.selection.is_empty() {
            self.move_to(self.next_boundary(), true, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }
    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_secret() || self.disabled || self.selection.is_empty() {
            return;
        }
        if let Some(text) = self.value.get(self.selection.clone()) {
            cx.write_to_clipboard(ClipboardItem::new_string(text.to_owned()));
        }
    }
    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_secret() || self.disabled || self.selection.is_empty() {
            return;
        }
        self.copy(&Copy, window, cx);
        self.replace_text_in_range(None, "", window, cx);
    }
    fn paste_clipboard(
        &mut self,
        _: &Paste,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(text) = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
        {
            self.replace_text_in_range(None, &text, window, cx);
        }
    }
    fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.is_secret() {
            return;
        }
        self.finish_composition();
        if let Some(snapshot) = self.undo.pop_back() {
            self.redo.push(self.snapshot());
            self.restore(snapshot, cx);
        }
    }
    fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.is_secret() {
            return;
        }
        self.finish_composition();
        if let Some(snapshot) = self.redo.pop() {
            self.undo.push_back(self.snapshot());
            self.restore(snapshot, cx);
        }
    }
    fn restore(&mut self, snapshot: EditSnapshot, cx: &mut Context<Self>) {
        self.value = snapshot.value;
        self.selection = snapshot.selection;
        self.reversed = snapshot.reversed;
        self.changed(cx);
    }
    fn enter(&mut self, secondary: bool, shift: bool, cx: &mut Context<Self>) {
        if !self.disabled && self.marked.is_none() {
            cx.emit(InputEvent::PressEnter { secondary, shift });
        }
    }

    fn mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        self.focus(window, cx);
        self.selecting = true;
        let offset = self.index_for_position(event.position);
        if event.click_count >= 3 || (event.click_count == 2 && self.is_secret()) {
            self.select_all(&SelectAll, window, cx);
        } else if event.click_count == 2 {
            let range = self
                .value
                .split_word_bound_indices()
                .find_map(|(start, word)| {
                    (start <= offset && offset < start + word.len())
                        .then_some(start..start + word.len())
                });
            if let Some(range) = range {
                self.move_to(range.start, false, cx);
                self.move_to(range.end, true, cx);
            }
        } else {
            self.move_to(offset, event.modifiers.shift, cx);
        }
        cx.stop_propagation();
    }
    fn mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.selecting = false;
    }
    fn mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selecting && !self.disabled {
            self.move_to(self.index_for_position(event.position), true, cx);
        }
    }

    pub(crate) fn display_offset(&self, offset: usize) -> usize {
        if !self.is_secret() {
            return offset;
        }
        self.value
            .grapheme_indices(true)
            .take_while(|(start, _)| *start < offset)
            .count()
            * '•'.len_utf8()
    }
    fn value_offset(&self, offset: usize) -> usize {
        if !self.is_secret() {
            return floor_boundary(&self.value, offset);
        }
        self.value
            .grapheme_indices(true)
            .nth(offset / '•'.len_utf8())
            .map(|(index, _)| index)
            .unwrap_or(self.value.len())
    }
    fn index_for_position(&self, position: Point<Pixels>) -> usize {
        self.layout
            .as_ref()
            .map(|line| {
                self.value_offset(
                    line.closest_index_for_x(position.x - self.text_origin.x),
                )
            })
            .unwrap_or(0)
    }
    fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        let start = from_utf16(&self.value, range.start);
        start..from_utf16(&self.value, range.end).max(start)
    }
    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        to_utf16(&self.value, range.start)..to_utf16(&self.value, range.end)
    }
    fn replacement_range(&self, range: Option<Range<usize>>) -> Range<usize> {
        range
            .map(|range| self.range_from_utf16(&range))
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.selection.clone())
    }
    fn replace(&mut self, range: Range<usize>, text: &str) {
        let mut value = self.value.to_string();
        value.replace_range(range.clone(), text);
        self.value = value.into();
        let end = range.start + text.len();
        self.selection = end..end;
        self.reversed = false;
    }
}

impl EventEmitter<InputEvent> for InputState {}
impl Focusable for InputState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EntityInputHandler for InputState {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range);
        *actual = Some(self.range_to_utf16(&range));
        self.value.get(range).map(str::to_owned)
    }
    fn selected_text_range(
        &mut self,
        ignore_disabled: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        (ignore_disabled || !self.disabled).then(|| UTF16Selection {
            range: self.range_to_utf16(&self.selection),
            reversed: self.reversed,
        })
    }
    fn marked_text_range(
        &self,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.finish_composition();
        cx.notify();
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        let range = self.replacement_range(range);
        let text = normalize(text);
        if self.marked.is_none() && self.value.get(range.clone()) == Some(text.as_str()) {
            return;
        }
        if self.composition.is_none() {
            self.push_undo(self.snapshot());
        }
        self.replace(range, &text);
        self.finish_composition();
        self.changed(cx);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        let range = self.replacement_range(range);
        let text = normalize(text);
        if self.composition.is_none() && !self.is_secret() {
            self.composition = Some(self.snapshot());
        }
        self.replace(range.clone(), &text);
        self.marked = (!text.is_empty()).then_some(range.start..range.start + text.len());
        if let Some(selected) = selected {
            // IME selection offsets are relative to the replacement, not the whole value.
            let start = from_utf16(&text, selected.start);
            self.selection = range.start + start
                ..range.start + from_utf16(&text, selected.end).max(start);
        }
        if text.is_empty() {
            self.finish_composition();
        }
        self.changed(cx);
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range);
        let line = self.layout.as_ref()?;
        let bounds = self.bounds?;
        Some(Bounds::from_corners(
            point(
                self.text_origin.x + line.x_for_index(self.display_offset(range.start)),
                bounds.top(),
            ),
            point(
                self.text_origin.x + line.x_for_index(self.display_offset(range.end)),
                bounds.bottom(),
            ),
        ))
    }
    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        self.bounds
            .filter(|bounds| bounds.contains(&position))?;
        Some(to_utf16(&self.value, self.index_for_position(position)))
    }
    fn accepts_text_input(&self, _: &mut Window, _: &mut Context<Self>) -> bool {
        !self.disabled
    }
    fn text_length_utf16(
        &mut self,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.value.encode_utf16().count())
    }
    fn set_selected_text_range(
        &mut self,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        self.selection = self.range_from_utf16(&range);
        self.reversed = false;
        self.pause_blink(cx);
        cx.notify();
    }
}

impl Render for InputState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("input-state")
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .key_context("GhostInput")
            .track_focus(&self.focus_handle)
            .role(if self.is_secret() {
                Role::PasswordInput
            } else {
                Role::TextInput
            })
            .aria_placeholder(self.placeholder.clone())
            .when(!self.is_secret(), |element| {
                element.aria_value(self.value.clone())
            })
            .cursor(if self.disabled {
                CursorStyle::Arrow
            } else {
                CursorStyle::IBeam
            })
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::select_home))
            .on_action(cx.listener(Self::select_end))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste_clipboard))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(
                cx.listener(|state, _: &Enter, _, cx| state.enter(false, false, cx)),
            )
            .on_action(cx.listener(|state, _: &SecondaryEnter, _, cx| {
                state.enter(true, false, cx)
            }))
            .on_action(
                cx.listener(|state, _: &ShiftEnter, _, cx| state.enter(false, true, cx)),
            )
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .child(TextElement { input: cx.entity() })
    }
}

fn normalize(text: &str) -> String {
    text.chars()
        .map(|character| {
            if matches!(character, '\n' | '\r' | '\t' | '\u{2028}' | '\u{2029}') {
                ' '
            } else {
                character
            }
        })
        .collect()
}
fn floor_boundary(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .take_while(|index| *index <= offset)
        .last()
        .unwrap_or(0)
}
fn from_utf16(text: &str, offset: usize) -> usize {
    let mut units = 0;
    for (index, character) in text.char_indices() {
        if units >= offset {
            return index;
        }
        units += character.len_utf16();
    }
    text.len()
}
fn to_utf16(text: &str, offset: usize) -> usize {
    text.char_indices()
        .take_while(|(index, _)| *index < offset)
        .map(|(_, character)| character.len_utf16())
        .sum()
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use ghost_shell_gpui::{Entity, TestAppContext, VisualTestContext};

    use super::*;

    fn input(cx: &mut TestAppContext) -> (Entity<InputState>, &mut VisualTestContext) {
        cx.update(ghost_shell_theme::init);
        let (input, window) = cx.add_window_view(|window, cx| {
            let input = InputState::new(window, cx);
            input.focus(window, cx);
            input
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        (input, window)
    }

    #[test]
    fn utf16_offsets_and_grapheme_boundaries() {
        let text = "a👩‍💻e\u{301}";
        assert_eq!(from_utf16(text, 1), 1);
        assert_eq!(from_utf16(text, 6), "a👩‍💻".len());
        assert_eq!(to_utf16(text, "a👩‍💻".len()), 6);
        assert_eq!(floor_boundary(text, 5), 1);
        assert_eq!(from_utf16(text, usize::MAX), text.len());
        assert_eq!(normalize("a\r\nb\tc\u{2028}"), "a  b c ");
    }

    #[ghost_shell_gpui::test]
    fn editing_uses_graphemes_and_restores_history(cx: &mut TestAppContext) {
        let (input, window) = input(cx);
        window.update(|window, cx| {
            input.update(cx, |input, cx| {
                input.set_value("a👩‍💻e\u{301}", window, cx);
                input.backspace(&Backspace, window, cx);
                assert_eq!(input.value(), "a👩‍💻");
                input.backspace(&Backspace, window, cx);
                assert_eq!(input.value(), "a");
                input.undo(&Undo, window, cx);
                assert_eq!(input.value(), "a👩‍💻");
                input.redo(&Redo, window, cx);
                assert_eq!(input.value(), "a");
                input.select_all(&SelectAll, window, cx);
                input.replace_text_in_range(None, "hello\nworld", window, cx);
                assert_eq!(input.value(), "hello world");
            })
        });
    }

    #[ghost_shell_gpui::test]
    fn ime_selection_is_relative_to_composition_and_undo_is_atomic(
        cx: &mut TestAppContext,
    ) {
        let (input, window) = input(cx);
        window.update(|window, cx| {
            input.update(cx, |input, cx| {
                input.set_value("prefix ", window, cx);
                input.replace_and_mark_text_in_range(None, "😀x", Some(2..3), window, cx);
                assert_eq!(input.value(), "prefix 😀x");
                assert_eq!(input.selection, 11..12);
                assert_eq!(input.marked, Some(7..12));
                input.replace_and_mark_text_in_range(
                    None,
                    "😀yz",
                    Some(2..4),
                    window,
                    cx,
                );
                assert_eq!(input.selection, 11..13);
                input.replace_text_in_range(None, "😀yz", window, cx);
                assert_eq!(input.marked, None);
                input.undo(&Undo, window, cx);
                assert_eq!(input.value(), "prefix ");
                input.redo(&Redo, window, cx);
                assert_eq!(input.value(), "prefix 😀yz");
                input.replace_text_in_range(
                    Some(usize::MAX..usize::MAX),
                    "!",
                    window,
                    cx,
                );
                assert_eq!(input.value(), "prefix 😀yz!");
            })
        });
    }

    #[ghost_shell_gpui::test]
    fn password_never_copies_cuts_or_keeps_history(cx: &mut TestAppContext) {
        let (input, window) = input(cx);
        window.update(|window, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string("safe".into()));
            input.update(cx, |input, cx| {
                input.configure(false, InputContentType::Password, window, cx);
                input.set_value("secret👩‍💻", window, cx);
                input.select_all(&SelectAll, window, cx);
                input.copy(&Copy, window, cx);
                input.cut(&Cut, window, cx);
                assert_eq!(input.value(), "secret👩‍💻");
                assert_eq!(input.display_offset(input.value.len()), 7 * '•'.len_utf8());
                input.replace_text_in_range(None, "new secret", window, cx);
                assert!(input.undo.is_empty());
                input.undo(&Undo, window, cx);
                assert_eq!(input.value(), "new secret");
                input.set_value("", window, cx);
                assert!(input.undo.is_empty());
                assert!(input.redo.is_empty());
            });
            assert_eq!(
                cx.read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref(),
                Some("safe")
            );
        });
    }

    #[ghost_shell_gpui::test]
    fn disabled_input_rejects_edits_and_stops_blinking(cx: &mut TestAppContext) {
        let (input, window) = input(cx);
        window.update(|window, cx| {
            input.update(cx, |input, cx| {
                input.set_value("original", window, cx);
                input.configure(true, InputContentType::Text, window, cx);
                input.replace_text_in_range(None, "changed", window, cx);
                input.replace_and_mark_text_in_range(None, "changed", None, window, cx);
                input.backspace(&Backspace, window, cx);
                assert_eq!(input.value(), "original");
                assert!(input.blink_task.is_none());
                assert!(!input.accepts_text_input(window, cx));
            })
        });
    }

    #[ghost_shell_gpui::test]
    fn keyboard_actions_emit_change_and_enter(cx: &mut TestAppContext) {
        let (input, window) = input(cx);
        let events = Rc::new(RefCell::new(Vec::new()));
        let subscription = window.update(|_, cx| {
            let events = events.clone();
            cx.subscribe(&input, move |_, event: &InputEvent, _| {
                events.borrow_mut().push(event.clone())
            })
        });
        window.simulate_input("abc");
        window.simulate_keystrokes("left backspace enter");
        window.update(|_, cx| assert_eq!(input.read(cx).value(), "ac"));
        assert!(events.borrow().contains(&InputEvent::Change));
        assert!(
            events
                .borrow()
                .contains(&InputEvent::PressEnter {
                    secondary: false,
                    shift: false
                })
        );
        drop(subscription);
    }

    #[ghost_shell_gpui::test]
    fn clearing_focus_cancels_cursor_timer(cx: &mut TestAppContext) {
        let (input, window) = input(cx);
        window.update(|window, cx| window.blur(cx));
        window.update(|_, cx| {
            assert!(input.read(cx).blink_task.is_none());
            assert!(!input.read(cx).cursor_visible);
        });
    }
}
