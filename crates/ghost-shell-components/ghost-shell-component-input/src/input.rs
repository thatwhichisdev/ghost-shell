// Adapted from GPUI Kit 0e63ea799766, copyright 2024 - 2026 Longbridge.
// Modified for Ghost Shell; see crates/ghost-shell-components/UPSTREAM.md.

use ghost_shell_gpui::{
    App, Entity, Global, IntoElement, KeyBinding, RenderOnce, StyleRefinement, Styled,
    Window, actions, div, prelude::*,
};
use ghost_shell_theme::{ActiveTheme, Sizable, Size};

mod element;
mod state;
pub use state::InputState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputEvent {
    Change,
    PressEnter { secondary: bool, shift: bool },
    Focus,
    Blur,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputContentType {
    #[default]
    Text,
    Password,
    NewPassword,
}

actions!(
    ghost_input,
    [
        Backspace,
        Delete,
        MoveLeft,
        MoveRight,
        MoveUp,
        MoveDown,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        SelectHome,
        SelectEnd,
        Copy,
        Cut,
        Paste,
        Undo,
        Redo,
        Enter,
        SecondaryEnter,
        ShiftEnter,
    ]
);

struct Initialized;
impl Global for Initialized {}

pub fn init(cx: &mut App) {
    if cx.has_global::<Initialized>() {
        return;
    }
    cx.set_global(Initialized);
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("GhostInput")),
        KeyBinding::new("delete", Delete, Some("GhostInput")),
        KeyBinding::new("left", MoveLeft, Some("GhostInput")),
        KeyBinding::new("right", MoveRight, Some("GhostInput")),
        KeyBinding::new("up", MoveUp, Some("GhostInput")),
        KeyBinding::new("down", MoveDown, Some("GhostInput")),
        KeyBinding::new("shift-left", SelectLeft, Some("GhostInput")),
        KeyBinding::new("shift-right", SelectRight, Some("GhostInput")),
        KeyBinding::new("home", Home, Some("GhostInput")),
        KeyBinding::new("end", End, Some("GhostInput")),
        KeyBinding::new("shift-home", SelectHome, Some("GhostInput")),
        KeyBinding::new("shift-end", SelectEnd, Some("GhostInput")),
        KeyBinding::new("ctrl-a", SelectAll, Some("GhostInput")),
        KeyBinding::new("ctrl-c", Copy, Some("GhostInput")),
        KeyBinding::new("ctrl-x", Cut, Some("GhostInput")),
        KeyBinding::new("ctrl-v", Paste, Some("GhostInput")),
        KeyBinding::new("ctrl-z", Undo, Some("GhostInput")),
        KeyBinding::new("ctrl-shift-z", Redo, Some("GhostInput")),
        KeyBinding::new("ctrl-y", Redo, Some("GhostInput")),
        KeyBinding::new("enter", Enter, Some("GhostInput")),
        KeyBinding::new("ctrl-enter", SecondaryEnter, Some("GhostInput")),
        KeyBinding::new("shift-enter", ShiftEnter, Some("GhostInput")),
    ]);
}

#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
    style: StyleRefinement,
    size: Size,
    appearance: bool,
    disabled: bool,
    content_type: InputContentType,
}

impl Input {
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            size: Size::Medium,
            appearance: true,
            disabled: false,
            content_type: InputContentType::Text,
        }
    }

    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn content_type(mut self, content_type: InputContentType) -> Self {
        self.content_type = content_type;
        self
    }
}

impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Input {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state.update(cx, |state, cx| {
            state.configure(self.disabled, self.content_type, window, cx);
        });
        let focused = self
            .state
            .read(cx)
            .focus_handle
            .is_focused(window);
        div()
            .w_full()
            .min_w_0()
            .flex()
            .items_center()
            .text_color(if self.disabled {
                cx.theme().muted_foreground
            } else {
                cx.theme().foreground
            })
            .when(self.appearance, |element| {
                element
                    .px(self.size.input_px())
                    .py(self.size.input_py())
                    .min_h(self.size.row_height())
                    .rounded(cx.theme().tokens.radius.md)
                    .bg(cx.theme().input)
                    .border_1()
                    .border_color(if focused && !self.disabled {
                        cx.theme().ring
                    } else {
                        cx.theme().border
                    })
            })
            .map(|mut element| {
                element.style().refine(&self.style);
                element
            })
            .child(self.state)
    }
}
