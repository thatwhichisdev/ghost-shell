use iced::{
    Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget,
        layout::{Limits, Node},
        mouse, overlay, renderer,
        widget::{Tree, tree},
    },
};
use iced_aw::menu::{Item, Menu, MenuBar};

pub(super) struct Popup<'a, Message, R = Renderer>
where
    R: renderer::Renderer,
{
    menu: Element<'a, Message, Theme, R>,
    canvas: Size,
    generation: u64,
    on_resize: fn(u32, u32) -> Message,
    on_dismiss: Message,
}

#[derive(Default)]
struct State {
    opened: bool,
    generation: u64,
    reported: Option<(u32, u32)>,
}

impl<'a, Message: Clone + 'a, R: renderer::Renderer + 'a> Popup<'a, Message, R> {
    pub(super) fn new(
        menu: Menu<'a, Message, Theme, R>,
        width: f32,
        depth: usize,
        generation: u64,
        on_resize: fn(u32, u32) -> Message,
        on_dismiss: Message,
    ) -> Self {
        let menu = MenuBar::new(vec![Item::with_menu(
            iced::widget::Space::new().width(1).height(1),
            menu,
        )])
        .safe_bounds_margin(f32::MAX)
        .close_on_item_click_global(false)
        .close_on_background_click_global(false)
        .style(|theme: &Theme, _| menu_style(theme));
        Self {
            menu: menu.into(),
            canvas: Size::new((depth.max(1) as f32 + 1.0) * (width + 20.0), 702.0),
            generation,
            on_resize,
            on_dismiss,
        }
    }
}

fn menu_style(theme: &Theme) -> iced_aw::menu::Style {
    iced_aw::menu::Style {
        bar_background: iced::Color::TRANSPARENT.into(),
        menu_background: theme.palette().background.into(),
        menu_border: iced::Border {
            color: theme
                .extended_palette()
                .background
                .strong
                .color,
            width: 1.0,
            radius: 6.0.into(),
        },
        menu_shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

impl<Message: Clone, R: renderer::Renderer> Widget<Message, Theme, R>
    for Popup<'_, Message, R>
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.menu)]
    }
    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        if state.generation != self.generation {
            *state = State {
                generation: self.generation,
                ..Default::default()
            };
            tree.children = self.children();
        } else {
            tree.diff_children(&[&self.menu]);
        }
    }
    fn layout(&mut self, tree: &mut Tree, renderer: &R, limits: &Limits) -> Node {
        let Some(child) = tree.children.first_mut() else {
            return Node::new(limits.max());
        };
        let anchor = self
            .menu
            .as_widget_mut()
            .layout(child, renderer, &Limits::new(Size::ZERO, self.canvas))
            .move_to(Point::new(self.canvas.width - 5.0, 0.0));
        let state = tree.state.downcast_mut::<State>();
        if !state.opened {
            // 0.14.1 has no controlled-open API. Click only the inert, hidden root;
            // all real menu input and submenu state remain owned by iced_aw.
            let mut messages = Vec::new();
            for event in [
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ] {
                self.menu.as_widget_mut().update(
                    child,
                    &event,
                    Layout::new(&anchor),
                    mouse::Cursor::Available(anchor.bounds().center()),
                    renderer,
                    &mut iced::advanced::clipboard::Null,
                    &mut Shell::new(&mut messages),
                    &Rectangle::new(Point::ORIGIN, self.canvas),
                );
            }
            state.opened = true;
        }
        Node::with_children(limits.max(), vec![anchor])
    }
    fn draw(
        &self,
        _: &Tree,
        _: &mut R,
        _: &Theme,
        _: &renderer::Style,
        _: Layout<'_>,
        _: mouse::Cursor,
        _: &Rectangle,
    ) {
    }
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &R,
        _: &Rectangle,
        _: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, R>> {
        let state = tree.state.downcast_mut::<State>();
        let child = tree.children.first_mut()?;
        let viewport = Rectangle::new(Point::ORIGIN, self.canvas);
        let inner = self.menu.as_widget_mut().overlay(
            child,
            layout.children().next()?,
            renderer,
            &viewport,
            Vector::ZERO,
        )?;
        Some(overlay::Element::new(Box::new(PopupOverlay {
            inner,
            canvas: self.canvas,
            reported: &mut state.reported,
            measured: (258, 38),
            on_resize: self.on_resize,
            on_dismiss: self.on_dismiss.clone(),
        })))
    }
}
impl<'a, Message: Clone + 'a, R: renderer::Renderer + 'a> From<Popup<'a, Message, R>>
    for Element<'a, Message, Theme, R>
{
    fn from(popup: Popup<'a, Message, R>) -> Self {
        Self::new(popup)
    }
}

struct PopupOverlay<'a, Message, R: renderer::Renderer> {
    inner: overlay::Element<'a, Message, Theme, R>,
    canvas: Size,
    reported: &'a mut Option<(u32, u32)>,
    measured: (u32, u32),
    on_resize: fn(u32, u32) -> Message,
    on_dismiss: Message,
}

// This is the layout contract of iced_aw 0.14.1's MenuBarOverlay:
// [bar, roots, menus], with each menu containing [visible rows, items, offset].
// Pin the version and test this bridge when upgrading; it lets the native popup
// follow actual open menus instead of reserving space for every possible branch.
fn menu_bounds(layout: Layout<'_>) -> impl Iterator<Item = Rectangle> {
    layout
        .children()
        .nth(2)
        .into_iter()
        .flat_map(|menus| menus.children())
        .filter_map(|menu| {
            menu.children().nth(1).map(|items| {
                let bounds = items.bounds();
                Rectangle {
                    x: bounds.x - 4.0,
                    y: bounds.y - 4.0,
                    width: bounds.width + 8.0,
                    height: bounds.height + 8.0,
                }
            })
        })
}
impl<Message: Clone, R: renderer::Renderer> overlay::Overlay<Message, Theme, R>
    for PopupOverlay<'_, Message, R>
{
    fn layout(&mut self, renderer: &R, bounds: Size) -> Node {
        let node = self
            .inner
            .as_overlay_mut()
            .layout(renderer, self.canvas);
        let viewport = Rectangle::new(Point::ORIGIN, self.canvas);
        let visible = menu_bounds(Layout::new(&node))
            .filter_map(|rect| rect.intersection(&viewport))
            .reduce(|left, right| left.union(&right))
            .unwrap_or(Rectangle::new(Point::ORIGIN, Size::new(258.0, 38.0)));
        self.measured = (visible.width.ceil() as u32, visible.height.ceil() as u32);
        let translation = Vector::new(-visible.x, -visible.y);
        let content = Node::with_children(
            visible.size(),
            node.children()
                .iter()
                .cloned()
                .map(|child| child.translate(translation))
                .collect(),
        );
        Node::with_children(bounds, vec![content])
    }

    fn draw(
        &self,
        renderer: &mut R,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        if let Some(content) = layout.children().next() {
            self.inner
                .as_overlay()
                .draw(renderer, theme, style, content, cursor);
        }
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &R,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        if *self.reported != Some(self.measured) {
            *self.reported = Some(self.measured);
            shell.publish((self.on_resize)(self.measured.0, self.measured.1));
        }
        let Some(content) = layout.children().next() else {
            return;
        };
        if matches!(
            event,
            Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                ..
            })
        ) || (matches!(event, Event::Mouse(mouse::Event::ButtonPressed(_)))
            && !menu_bounds(content).any(|bounds| cursor.is_over(bounds)))
        {
            shell.publish(self.on_dismiss.clone());
            shell.capture_event();
            return;
        }
        // Native resizing is driven by the menu itself; upstream would otherwise
        // close the menu on every compositor resize acknowledgement.
        if !matches!(event, Event::Window(iced::window::Event::Resized { .. })) {
            self.inner
                .as_overlay_mut()
                .update(event, content, cursor, renderer, clipboard, shell);
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &R,
    ) -> mouse::Interaction {
        layout
            .children()
            .next()
            .map(|content| {
                self.inner
                    .as_overlay()
                    .mouse_interaction(content, cursor, renderer)
            })
            .unwrap_or_default()
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &R,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        if let Some(content) = layout.children().next() {
            self.inner
                .as_overlay_mut()
                .operate(content, renderer, operation);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    enum TestMessage {
        Resize(u32, u32),
        Dismiss,
        Activate(i32),
        Volume(f32),
    }

    #[test]
    fn menu_background_uses_the_exact_surface_color() {
        for background in [iced::Color::from_rgb8(10, 12, 16), iced::Color::WHITE] {
            let theme = Theme::custom(
                "Menu regression",
                iced::theme::Palette {
                    background,
                    ..Theme::Dark.palette()
                },
            );
            assert_ne!(
                background,
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
            );
            assert_eq!(menu_style(&theme).menu_background, background.into());
        }
    }

    fn audio_menu(rows: usize) -> Popup<'static, TestMessage, ()> {
        let mut items = vec![
            Item::new(
                iced::widget::slider(0.0..=1.0, 0.5, TestMessage::Volume)
                    .width(380)
                    .height(22),
            )
            .close_on_click(false),
        ];
        items.extend((0..rows).map(|_| {
            Item::new(
                iced::widget::Space::new()
                    .width(380)
                    .height(30),
            )
            .close_on_click(false)
        }));
        Popup::new(
            Menu::new(items)
                .width(380)
                .padding(4)
                .offset(8.0),
            380.0,
            1,
            0,
            TestMessage::Resize,
            TestMessage::Dismiss,
        )
    }

    #[test]
    fn audio_menu_grows_with_content_and_caps_height_for_scrolling() {
        let mut popup = audio_menu(2);
        let mut tree = Tree::new(&popup as &dyn Widget<TestMessage, Theme, ()>);
        let mut previous_height = 0.0;
        for rows in [2, 8, 40] {
            popup = audio_menu(rows);
            popup.diff(&mut tree);
            let size = Size::new(388.0, 702.0);
            let node = popup.layout(&mut tree, &(), &Limits::new(Size::ZERO, size));
            let viewport = Rectangle::new(Point::ORIGIN, size);
            let mut overlay = popup
                .overlay(&mut tree, Layout::new(&node), &(), &viewport, Vector::ZERO)
                .unwrap();
            let node = overlay.as_overlay_mut().layout(&(), size);
            let content = Layout::new(&node)
                .children()
                .next()
                .unwrap()
                .bounds();
            assert_eq!(content.width, 388.0);
            assert!(content.height > previous_height);
            assert!(content.height <= 702.0);
            if rows < 40 {
                assert_eq!(content.height, 30.0 * rows as f32 + 30.0);
            }
            previous_height = content.height;
        }
    }

    #[test]
    fn dragging_volume_keeps_menu_open_and_emits_changes() {
        let mut popup = audio_menu(2);
        let mut tree = Tree::new(&popup as &dyn Widget<TestMessage, Theme, ()>);
        let size = Size::new(388.0, 90.0);
        let node = popup.layout(&mut tree, &(), &Limits::new(Size::ZERO, size));
        let viewport = Rectangle::new(Point::ORIGIN, size);
        let mut overlay = popup
            .overlay(&mut tree, Layout::new(&node), &(), &viewport, Vector::ZERO)
            .unwrap();
        let node = overlay.as_overlay_mut().layout(&(), size);
        let position = Point::new(290.0, 15.0);
        let mut messages = Vec::new();
        for event in [
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::CursorMoved { position }),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ] {
            overlay.as_overlay_mut().update(
                &event,
                Layout::new(&node),
                mouse::Cursor::Available(position),
                &(),
                &mut iced::advanced::clipboard::Null,
                &mut Shell::new(&mut messages),
            );
        }
        assert!(messages.iter().any(
            |message| matches!(message, TestMessage::Volume(value) if *value > 0.5)
        ));
        assert!(
            !messages
                .iter()
                .any(|message| matches!(message, TestMessage::Dismiss))
        );
        let node = overlay.as_overlay_mut().layout(&(), size);
        assert_eq!(
            menu_bounds(Layout::new(&node).children().next().unwrap()).count(),
            1
        );
    }

    fn popup() -> Popup<'static, TestMessage, ()> {
        let leaf = |id| {
            Item::new(
                iced::widget::button("Action")
                    .width(250)
                    .height(30)
                    .on_press(TestMessage::Activate(id)),
            )
            .close_on_click(false)
        };
        Popup::new(
            Menu::new(vec![
                Item::with_menu(
                    iced::widget::Space::new()
                        .width(250)
                        .height(30),
                    Menu::new(vec![leaf(2)])
                        .width(250)
                        .padding(4)
                        .offset(8.0),
                ),
                leaf(3),
            ])
            .width(250)
            .padding(4)
            .offset(8.0),
            250.0,
            2,
            0,
            TestMessage::Resize,
            TestMessage::Dismiss,
        )
    }

    #[test]
    fn opens_without_second_click_and_resizes_only_for_open_submenus() {
        let mut popup = popup();
        let mut tree = Tree::new(&popup as &dyn Widget<TestMessage, Theme, ()>);
        let node = popup.layout(
            &mut tree,
            &(),
            &Limits::new(Size::ZERO, Size::new(258.0, 68.0)),
        );
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(258.0, 68.0));
        let mut overlay = popup
            .overlay(&mut tree, Layout::new(&node), &(), &viewport, Vector::ZERO)
            .unwrap();
        let first = overlay
            .as_overlay_mut()
            .layout(&(), viewport.size());
        let content = Layout::new(&first)
            .children()
            .next()
            .unwrap();
        let root = menu_bounds(content).next().unwrap();
        assert_eq!(root.width, 258.0);
        assert_eq!(root.x, 0.0);
        assert_eq!(menu_bounds(content).count(), 1);
        let mut messages = Vec::new();
        let position = Point::new(20.0, 15.0);
        overlay.as_overlay_mut().update(
            &Event::Mouse(mouse::Event::CursorMoved { position }),
            Layout::new(&first),
            mouse::Cursor::Available(position),
            &(),
            &mut iced::advanced::clipboard::Null,
            &mut Shell::new(&mut messages),
        );
        let expanded = overlay
            .as_overlay_mut()
            .layout(&(), Size::new(516.0, 68.0));
        let menus: Vec<_> = menu_bounds(
            Layout::new(&expanded)
                .children()
                .next()
                .unwrap(),
        )
        .collect();
        assert_eq!(menus.len(), 2);
        assert!(menus[1].x < menus[0].x, "submenus must expand left");
        assert_eq!(menus[0].x + menus[0].width, 516.0);
        // Resize acknowledgements must not close iced_aw's menu.
        overlay.as_overlay_mut().update(
            &Event::Window(iced::window::Event::Resized(Size::new(516.0, 68.0))),
            Layout::new(&expanded),
            mouse::Cursor::Unavailable,
            &(),
            &mut iced::advanced::clipboard::Null,
            &mut Shell::new(&mut messages),
        );
        let resized = overlay
            .as_overlay_mut()
            .layout(&(), Size::new(516.0, 68.0));
        assert_eq!(
            menu_bounds(
                Layout::new(&resized)
                    .children()
                    .next()
                    .unwrap()
            )
            .count(),
            2
        );
        assert!(
            messages
                .iter()
                .any(|message| matches!(message, TestMessage::Resize(516, 68)))
        );
        let position = menus[1].center();
        for event in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            overlay.as_overlay_mut().update(
                &Event::Mouse(event),
                Layout::new(&resized),
                mouse::Cursor::Available(position),
                &(),
                &mut iced::advanced::clipboard::Null,
                &mut Shell::new(&mut messages),
            );
        }
        assert!(
            messages
                .iter()
                .any(|message| matches!(message, TestMessage::Activate(2)))
        );
        let position = Point::new(menus[0].center_x(), 49.0);
        overlay.as_overlay_mut().update(
            &Event::Mouse(mouse::Event::CursorMoved { position }),
            Layout::new(&resized),
            mouse::Cursor::Available(position),
            &(),
            &mut iced::advanced::clipboard::Null,
            &mut Shell::new(&mut messages),
        );
        let collapsed = overlay
            .as_overlay_mut()
            .layout(&(), Size::new(258.0, 68.0));
        assert_eq!(
            menu_bounds(
                Layout::new(&collapsed)
                    .children()
                    .next()
                    .unwrap()
            )
            .count(),
            1
        );
        drop(overlay);
        popup.generation += 1;
        popup.diff(&mut tree);
        let node =
            popup.layout(&mut tree, &(), &Limits::new(Size::ZERO, viewport.size()));
        let mut overlay = popup
            .overlay(&mut tree, Layout::new(&node), &(), &viewport, Vector::ZERO)
            .unwrap();
        let refreshed = overlay
            .as_overlay_mut()
            .layout(&(), viewport.size());
        assert_eq!(
            menu_bounds(
                Layout::new(&refreshed)
                    .children()
                    .next()
                    .unwrap()
            )
            .count(),
            1
        );
    }
    #[test]
    fn escape_and_clicking_empty_popup_space_dismiss_without_activation() {
        use iced::keyboard::{self, Key, key};
        let escape = Event::Keyboard(keyboard::Event::KeyPressed {
            key: Key::Named(key::Named::Escape),
            modified_key: Key::Named(key::Named::Escape),
            physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: None,
            repeat: false,
        });
        for event in [
            escape,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        ] {
            let mut popup = popup();
            let mut tree = Tree::new(&popup as &dyn Widget<TestMessage, Theme, ()>);
            let size = Size::new(300.0, 100.0);
            let node = popup.layout(&mut tree, &(), &Limits::new(Size::ZERO, size));
            let viewport = Rectangle::new(Point::ORIGIN, size);
            let mut overlay = popup
                .overlay(&mut tree, Layout::new(&node), &(), &viewport, Vector::ZERO)
                .unwrap();
            let node = overlay.as_overlay_mut().layout(&(), size);
            let mut messages = Vec::new();
            overlay.as_overlay_mut().update(
                &event,
                Layout::new(&node),
                mouse::Cursor::Available(Point::new(280.0, 90.0)),
                &(),
                &mut iced::advanced::clipboard::Null,
                &mut Shell::new(&mut messages),
            );
            assert!(
                messages
                    .iter()
                    .any(|message| matches!(message, TestMessage::Dismiss))
            );
            assert!(
                !messages
                    .iter()
                    .any(|message| matches!(message, TestMessage::Activate(_)))
            );
        }
    }
}
