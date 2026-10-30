use iced::{
    Element, Event, Fill, Length, Rectangle, Renderer, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    widget::container,
};

pub const CONTENT_HEIGHT: f32 = 20.0;
pub const ICON_SIZE: f32 = 16.0;
const HORIZONTAL_MARGIN: f32 = 2.0;
const VERTICAL_MARGIN: f32 = 4.0;

pub fn outer<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(content)
        .padding([VERTICAL_MARGIN, HORIZONTAL_MARGIN])
        .center_y(Fill)
        .into()
}

pub fn inner<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    group_inner(content, true, true)
}

pub fn group_inner<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    first: bool,
    last: bool,
) -> Element<'a, Message> {
    Element::new(Hover {
        radius: iced::border::Radius {
            top_left: if first { 4.0 } else { 0.0 },
            bottom_left: if first { 4.0 } else { 0.0 },
            top_right: if last { 4.0 } else { 0.0 },
            bottom_right: if last { 4.0 } else { 0.0 },
        },
        content: container(content)
            .padding([0, 4])
            .center_y(CONTENT_HEIGHT)
            .into(),
    })
}

pub fn transparent_button(
    theme: &Theme,
    _: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        text_color: theme
            .extended_palette()
            .background
            .neutral
            .color,
        ..Default::default()
    }
}

struct Hover<'a, Message> {
    radius: iced::border::Radius,
    content: Element<'a, Message>,
}

impl<Message> Widget<Message, Theme, Renderer> for Hover<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<bool>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(false)
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let hovered = cursor.is_over(layout.bounds()) && cursor.is_over(*viewport);
        let previous = tree.state.downcast_mut::<bool>();
        if *previous != hovered {
            *previous = hovered;
            shell.request_redraw();
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let pair = theme.extended_palette().background.neutral;
        let hovered = cursor.is_over(layout.bounds()) && cursor.is_over(*viewport);
        container::draw_background(
            renderer,
            &container::Style::default()
                .background(if hovered {
                    theme
                        .extended_palette()
                        .secondary
                        .strong
                        .color
                } else {
                    theme.palette().background
                })
                .border(iced::Border {
                    radius: self.radius,
                    ..Default::default()
                }),
            layout.bounds(),
        );
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: pair.color,
            },
            layout,
            cursor,
            viewport,
        );
    }
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

pub fn svg<'a, Message: 'a>(handle: iced::advanced::svg::Handle) -> Element<'a, Message> {
    Element::new(Symbol(handle))
}

struct Symbol(iced::advanced::svg::Handle);

impl<Message> Widget<Message, Theme, Renderer> for Symbol {
    fn size(&self) -> Size<Length> {
        Size::new(ICON_SIZE.into(), ICON_SIZE.into())
    }
    fn layout(
        &mut self,
        _: &mut Tree,
        _: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(ICON_SIZE, ICON_SIZE, Size::ZERO))
    }
    fn draw(
        &self,
        _: &Tree,
        renderer: &mut Renderer,
        _: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _: mouse::Cursor,
        _: &Rectangle,
    ) {
        use iced::advanced::svg::{Renderer as _, Svg};
        let dimensions = renderer.measure_svg(&self.0);
        let size = iced::ContentFit::Contain.fit(
            Size::new(dimensions.width as f32, dimensions.height as f32),
            layout.bounds().size(),
        );
        let bounds = Rectangle::new(
            iced::Point::new(
                layout.bounds().center_x() - size.width / 2.0,
                layout.bounds().center_y() - size.height / 2.0,
            ),
            size,
        );
        renderer.draw_svg(
            Svg::new(self.0.clone()).color(style.text_color),
            bounds,
            layout.bounds(),
        );
    }
}

pub fn icon<'a, Message: 'a>(name: &str) -> Element<'a, Message> {
    match ghost_shell_assets::Assets::get(name) {
        Some(asset) => svg(iced::advanced::svg::Handle::from_memory(
            asset.data.into_owned(),
        )),
        None => iced::widget::text("•").into(),
    }
}
