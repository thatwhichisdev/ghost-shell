use std::rc::Rc;

use ghost_shell_component_icon::{Icon, IconName};
use ghost_shell_component_input::{Input, InputContentType, InputEvent, InputState};
use ghost_shell_component_menu::{PopupMenu, PopupMenuItem};
use ghost_shell_component_root::Root;
use ghost_shell_component_spinner::Spinner;
use ghost_shell_component_virtual_list::{VirtualListScrollHandle, v_virtual_list};
use ghost_shell_gpui::{prelude::*, *};
use ghost_shell_theme::{ActiveTheme, Sizable, Theme, ThemeMode};

struct Preview {
    query: Entity<InputState>,
    password: Entity<InputState>,
    submitted: SharedString,
    menu: Option<Entity<PopupMenu>>,
    menu_subscription: Option<Subscription>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    scroll: VirtualListScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl Preview {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| {
            let input =
                InputState::new(window, cx).placeholder("Type a query, then press Enter");
            input.focus(window, cx);
            input
        });
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Password (no copy/cut)")
                .masked(true)
        });
        let subscription = cx.subscribe(&query, |preview, input, event, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                preview.submitted = input.read(cx).value();
                cx.notify();
            }
        });
        Self {
            query,
            password,
            submitted: "".into(),
            menu: None,
            menu_subscription: None,
            item_sizes: Rc::new(
                (0..1000)
                    .map(|index| size(px(0.), px(if index % 3 == 0 { 40. } else { 28. })))
                    .collect(),
            ),
            scroll: VirtualListScrollHandle::new(),
            _subscriptions: vec![subscription],
        }
    }

    fn open_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let menu = PopupMenu::build(window, cx, |menu, window, cx| {
            menu.item(PopupMenuItem::new("Example checked item").checked(true))
                .item(PopupMenuItem::new("Disabled item").disabled(true))
                .separator()
                .submenu("Appearance", window, cx, |menu, _, _| {
                    menu.item(
                        PopupMenuItem::new("Dark").on_click(|_, _, cx| {
                            Theme::set(Theme::new(ThemeMode::Dark), cx)
                        }),
                    )
                    .item(PopupMenuItem::new("Light").on_click(|_, _, cx| {
                        Theme::set(Theme::new(ThemeMode::Light), cx)
                    }))
                })
        });
        self.menu_subscription =
            Some(cx.subscribe(&menu, |preview, _, _: &DismissEvent, cx| {
                preview.menu = None;
                preview.menu_subscription = None;
                cx.notify();
            }));
        menu.read(cx)
            .focus_handle(cx)
            .focus(window, cx);
        self.menu = Some(menu);
        cx.notify();
    }
}

impl Render for Preview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .relative()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(Icon::new(IconName::Check).large())
                    .child("Ghost components — self-contained GPUI")
                    .child(Spinner::new().small()),
            )
            .child(Input::new(&self.query))
            .child(Input::new(&self.password).content_type(InputContentType::Password))
            .child(format!("Submitted: {}", self.submitted))
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .id("menu-button")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(cx.theme().secondary)
                            .cursor_pointer()
                            .child("Open menu")
                            .on_click(cx.listener(|preview, _, window, cx| {
                                preview.open_menu(window, cx)
                            })),
                    )
                    .child(
                        div()
                            .id("scroll-button")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(cx.theme().secondary)
                            .cursor_pointer()
                            .child("Jump to row 900")
                            .on_click(cx.listener(|preview, _, _, cx| {
                                preview
                                    .scroll
                                    .scroll_to_item(900, ScrollStrategy::Top);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        v_virtual_list(
                            cx.entity(),
                            "preview-list",
                            self.item_sizes.clone(),
                            |preview, range, _, cx| {
                                range
                                    .map(|index| {
                                        div()
                                            .h(preview.item_sizes[index].height)
                                            .w_full()
                                            .flex()
                                            .items_center()
                                            .px_2()
                                            .when(index % 2 == 0, |row| {
                                                row.bg(cx.theme().surface)
                                            })
                                            .child(format!("Variable-height row {index}"))
                                    })
                                    .collect::<Vec<_>>()
                            },
                        )
                        .track_scroll(&self.scroll),
                    ),
            )
            .when_some(self.menu.clone(), |element, menu| {
                element.child(deferred(
                    anchored()
                        .position(point(px(30.), px(240.)))
                        .child(menu)
                        .snap_to_window_with_margin(Edges::all(px(8.))),
                ))
            })
    }
}

fn main() {
    ghost_shell_gpui::application().run(|cx| {
        ghost_shell_component_root::init(cx);
        Theme::set(Theme::new(ThemeMode::Dark), cx);
        if let Err(error) = cx.open_window(WindowOptions::default(), |window, cx| {
            let preview = cx.new(|cx| Preview::new(window, cx));
            cx.new(|cx| Root::new(preview, window, cx))
        }) {
            eprintln!("Failed to open component preview: {error:#}");
            cx.quit();
        }
    });
}
