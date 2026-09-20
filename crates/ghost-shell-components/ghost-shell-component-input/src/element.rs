// Adapted from GPUI Kit's input painter and GPUI's input example.
// See crates/ghost-shell-components/UPSTREAM.md for provenance.

use ghost_shell_gpui::{prelude::*, *};
use ghost_shell_theme::ActiveTheme;
use unicode_segmentation::UnicodeSegmentation;

use super::InputState;

pub(crate) struct TextElement {
    pub input: Entity<InputState>,
}

pub(crate) struct PrepaintState {
    line: Option<ShapedLine>,
    origin: Point<Pixels>,
    scroll_offset: Pixels,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> PrepaintState {
        let state = self.input.read(cx);
        let style = window.text_style();
        let empty = state.value.is_empty();
        let display: SharedString = if empty {
            state.placeholder.clone()
        } else if state.is_secret() {
            "•"
                .repeat(state.value.graphemes(true).count())
                .into()
        } else {
            state.value.clone()
        };
        let run = TextRun {
            len: display.len(),
            font: style.font(),
            color: if empty {
                cx.theme().muted_foreground
            } else {
                style.color
            },
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked) = &state.marked {
            let marked =
                state.display_offset(marked.start)..state.display_offset(marked.end);
            vec![
                TextRun {
                    len: marked.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked.len(),
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display.len().saturating_sub(marked.end),
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };
        let line = window.text_system().shape_line(
            display,
            style.font_size.to_pixels(window.rem_size()),
            &runs,
            None,
        );
        let cursor_position = line.x_for_index(state.display_offset(state.cursor()));
        let available_width = (bounds.size.width - px(2.)).max(px(0.));
        let mut scroll_offset = state
            .scroll_offset
            .min((line.width - available_width).max(px(0.)));
        if cursor_position < scroll_offset {
            scroll_offset = cursor_position;
        }
        if cursor_position > scroll_offset + available_width {
            scroll_offset = cursor_position - available_width;
        }
        let align_offset = if line.width <= available_width {
            match style.text_align {
                TextAlign::Center => (bounds.size.width - line.width) * 0.5,
                TextAlign::Right => bounds.size.width - line.width,
                _ => px(0.),
            }
        } else {
            px(0.)
        };
        let origin = point(bounds.left() + align_offset - scroll_offset, bounds.top());
        let focused = state.focus_handle.is_focused(window) && !state.disabled;
        let cursor = (focused && state.cursor_visible && state.selection.is_empty())
            .then(|| {
                fill(
                    Bounds::new(
                        point(origin.x + cursor_position, origin.y),
                        size(px(2.), bounds.size.height),
                    ),
                    cx.theme().foreground,
                )
            });
        let selection = (focused && !state.selection.is_empty()).then(|| {
            fill(
                Bounds::from_corners(
                    point(
                        origin.x
                            + line
                                .x_for_index(state.display_offset(state.selection.start)),
                        bounds.top(),
                    ),
                    point(
                        origin.x
                            + line.x_for_index(state.display_offset(state.selection.end)),
                        bounds.bottom(),
                    ),
                ),
                cx.theme().selection,
            )
        });
        PrepaintState {
            line: Some(line),
            origin,
            scroll_offset,
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        state: &mut PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let input = self.input.read(cx);
        if !input.disabled {
            window.handle_input(
                &input.focus_handle,
                ElementInputHandler::new(bounds, self.input.clone()),
                cx,
            );
        }
        if let Some(selection) = state.selection.take() {
            window.paint_quad(selection);
        }
        let Some(line) = state.line.take() else {
            return;
        };
        if let Err(error) = line.paint(
            state.origin,
            bounds.size.height,
            TextAlign::Left,
            None,
            window,
            cx,
        ) {
            log::error!("Failed to paint input text: {error:#}");
        }
        if let Some(cursor) = state.cursor.take() {
            window.paint_quad(cursor);
        }
        self.input.update(cx, |input, _| {
            input.layout = Some(line);
            input.bounds = Some(bounds);
            input.text_origin = state.origin;
            input.scroll_offset = state.scroll_offset;
        });
    }
}
