use anyhow::{Context as _, Result};
use ghost_shell_app::GhostShell;
use gpui::{
    AnyWindowHandle, App, Bounds, Context, Entity, Global, IntoElement, Render,
    Subscription, Window, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, div, img, prelude::*, px, size,
};
use gpui_component::{
    ActiveTheme as _, IndexPath, Root, Sizable, StyledExt,
    input::{Input, InputEvent, InputState, MoveDown, MoveUp},
    list::{List, ListDelegate, ListItem, ListState},
};
use neo_frizbee::Config;

use crate::{Application, Applications, Launch};

pub struct Launcher {
    handle: Option<AnyWindowHandle>,
}

impl Launcher {
    #[must_use]
    pub fn new() -> Self {
        Self { handle: None }
    }

    pub fn toggle(&mut self, cx: &mut App) -> Result<()> {
        if self.handle.is_some() {
            self.close(cx)
        } else {
            self.open(cx)
        }
    }

    pub fn open(&mut self, cx: &mut App) -> Result<()> {
        let ghost_shell = cx.global::<GhostShell>();

        let output = ghost_shell
            .get_focused_output()
            .unwrap_or(ghost_shell.get_primary_output());

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
            let view = cx.new(|cx| LauncherView::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx).bordered(false))
        })?;

        self.handle = Some(handle.into());

        Ok(())
    }

    pub fn close(&mut self, cx: &mut App) -> Result<()> {
        let Some(handle) = self.handle.take() else {
            return Ok(());
        };

        handle
            .update(cx, |_view, window, _cx| {
                window.remove_window();
            })
            .context("failed to close launcher window")?;

        self.handle = None;

        Ok(())
    }
}

struct LauncherView {
    /// A strong well-typed reference to launcher's input
    input: Entity<InputState>,

    /// A handle to a input's subscription.
    /// When dropped, the subscription is cancelled and the callback will no longer be invoked.
    #[allow(unused)]
    input_subscription: Subscription,

    /// A strong well-typed reference to launcher's list of applications and files
    list: Entity<ListState<ApplicationListDelegate>>,
}

impl LauncherView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let apps = cx.global::<Applications>().clone();

        // Initialize list of applications and select first item
        let list = cx.new(|cx| {
            let all_indices = (0..apps.items.len()).collect();
            let delegate = ApplicationListDelegate {
                all_items: apps.items,
                filtered_indices: all_indices,
                selected_index: None,
            };
            let mut list = ListState::new(delegate, window, cx);

            list.set_selected_index(Some(IndexPath::new(0)), window, cx);
            list
        });

        // Initialiaze input and focus it
        let input = cx.new(|cx| {
            let input = InputState::new(window, cx);

            input.focus(window, cx);
            input
        });

        // Subscirbe to input's events and keep the handle
        let input_subscription =
            cx.subscribe_in(&input, window, Self::on_query_event);

        Self {
            input,
            input_subscription,
            list,
        }
    }

    fn on_query_event(
        &mut self,
        input: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }

        let query = input.read(cx).value();
        let query = query.trim();

        self.list.update(cx, |list, cx| {
            let has_items = {
                let delegate = list.delegate_mut();
                delegate.filter(query);
                !delegate.filtered_indices.is_empty()
            };

            let selected = has_items.then_some(IndexPath::new(0));

            list.set_selected_index(selected, window, cx);
            cx.notify();
        });
    }

    fn select_next_item(
        &mut self,
        _: &MoveDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.list.update(cx, |list, cx| {
            let items_count = list.delegate().filtered_indices.len();
            let next_index = match (items_count, list.selected_index()) {
                (0, _) => None,
                (_, None) => Some(IndexPath::new(0)),
                (items_count, Some(current)) => {
                    let next_row = if current.row + 1 >= items_count {
                        0
                    } else {
                        current.row + 1
                    };

                    Some(IndexPath::new(next_row))
                }
            };

            if next_index.is_some() {
                list.set_selected_index(next_index, window, cx);
                list.scroll_to_selected_item(window, cx);
            }
        });
    }

    fn select_previous_item(
        &mut self,
        _: &MoveUp,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.list.update(cx, |list, cx| {
            let items_count = list.delegate().filtered_indices.len();
            let previous_index = match (items_count, list.selected_index()) {
                (0, _) => None,
                (items_count, None) => Some(IndexPath::new(items_count - 1)),
                (items_count, Some(current)) => {
                    let previous_row =
                        if current.row == 0 || current.row >= items_count {
                            items_count - 1
                        } else {
                            current.row - 1
                        };

                    Some(IndexPath::new(previous_row))
                }
            };

            if previous_index.is_some() {
                list.set_selected_index(previous_index, window, cx);
                list.scroll_to_selected_item(window, cx);
            }
        });
    }

    fn launch_selected_item(
        &mut self,
        _: &Launch,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // todo: implement app launching logic,
        // will need to figure out how to spawn a detached proccess using systemd,
        // or if not available just spawn a child process
        if let Some(application) = self.list.read(cx).delegate().selected_item()
        {
            println!("Selected application: {application:?}");
        }
    }
}

impl Render for LauncherView {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("launcher")
            .key_context("launcher")
            .size_full()
            .on_action(cx.listener(Self::select_previous_item))
            .on_action(cx.listener(Self::select_next_item))
            .on_action(cx.listener(Self::launch_selected_item))
            .flex()
            .flex_col()
            .bg(cx.theme().colors.background)
            .text_color(cx.theme().colors.foreground)
            .overflow_hidden()
            .child(
                div()
                    .id("launcher-input")
                    .w_full()
                    .p_3()
                    .border_b_1()
                    .child(Input::new(&self.input).large().cleanable(true)),
            )
            .child(
                div()
                    .id("launcher-results")
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(List::new(&self.list).size_full()),
            )
    }
}

impl Global for Launcher {}

struct ApplicationListDelegate {
    all_items: Vec<Application>,
    filtered_indices: Vec<usize>,
    selected_index: Option<IndexPath>,
}

impl ApplicationListDelegate {
    fn item(&self, row: usize) -> Option<&Application> {
        let item_index = *self.filtered_indices.get(row)?;
        self.all_items.get(item_index)
    }

    fn selected_item(&self) -> Option<&Application> {
        let selected = self.selected_index?;
        self.item(selected.row)
    }

    fn filter(&mut self, query: &str) {
        self.filtered_indices.clear();

        if query.is_empty() {
            self.filtered_indices.extend(0..self.all_items.len());
            return;
        }

        let matches =
            neo_frizbee::match_list(query, &self.all_items, &Config::default());

        self.filtered_indices.reserve(matches.len());
        self.filtered_indices
            .extend(matches.into_iter().map(|matched| matched.index as usize));
    }
}

impl ListDelegate for ApplicationListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.filtered_indices.len()
    }

    fn render_item(
        &mut self,
        index: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        const ICON_SIZE: f32 = 40.0;
        const ROW_HEIGHT: f32 = 48.0;

        let app = self.item(index.row)?;
        let selected = self.selected_index == Some(index);

        let title_color = if selected {
            cx.theme().accent_foreground
        } else {
            cx.theme().foreground
        };

        let description_color = if selected {
            cx.theme().accent_foreground
        } else {
            cx.theme().muted_foreground
        };

        let description = app
            .description
            .clone()
            .filter(|description| !description.trim().is_empty());

        let icon_column = div()
            .w(px(ICON_SIZE))
            .h(px(ICON_SIZE))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .when_some(app.icon.clone(), |this, icon_path| {
                this.child(
                    img(icon_path)
                        .size(px(ICON_SIZE))
                        .object_fit(gpui::ObjectFit::Contain),
                )
            });

        let text_column = div()
            .min_w_0()
            .flex_1()
            .flex()
            .flex_col()
            .justify_center()
            .overflow_hidden()
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .text_size(px(14.0))
                    .font_bold()
                    .text_color(title_color)
                    .truncate()
                    .child(app.name.clone()),
            )
            .when_some(description, |this, description| {
                this.child(
                    div()
                        .w_full()
                        .min_w_0()
                        .text_size(px(13.0))
                        .text_color(description_color)
                        .when(selected, |this| this.opacity(0.7))
                        .truncate()
                        .child(description),
                )
            });

        let content = div()
            .w_full()
            .min_w_0()
            .h_full()
            .flex()
            .items_center()
            .gap_x_3()
            .overflow_hidden()
            .child(icon_column)
            .child(text_column);

        Some(
            ListItem::new(index)
                .h(px(ROW_HEIGHT))
                // Override ListItem's default 12px horizontal padding.
                .px_2()
                .py_1()
                .rounded_md()
                .child(content)
                .selected(selected),
        )
    }

    fn set_selected_index(
        &mut self,
        index: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = index;
        cx.notify();
    }
}
