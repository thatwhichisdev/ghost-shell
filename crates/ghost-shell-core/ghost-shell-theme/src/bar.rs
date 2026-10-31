use iced::{
    Element, Event, Fill, Length, Rectangle, Renderer, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    widget::container,
};

#[derive(Clone, Copy)]
pub struct Style {
    pub font: iced::Font,
    pub font_size: f32,
    pub widgets: ghost_shell_config::BarWidgetsConfig,
    pub decoration: ghost_shell_config::BarDecorationConfig,
}

impl From<&ghost_shell_config::BarConfig> for Style {
    fn from(config: &ghost_shell_config::BarConfig) -> Self {
        Self {
            font: iced::Font::MONOSPACE,
            font_size: 13.0,
            widgets: config.widgets,
            decoration: config.decoration,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decoration {
    pub bounds: Rectangle,
    pub radius: iced::border::Radius,
}

#[derive(Default)]
struct State {
    hovered: bool,
    decorations: Option<Vec<Decoration>>,
}

pub fn outer<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    style: Style,
) -> Element<'a, Message> {
    container(content)
        .padding([
            style.widgets.margin_vertical,
            style.widgets.margin_horizontal,
        ])
        .center_y(Fill)
        .into()
}

pub fn inner<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    style: Style,
) -> Element<'a, Message> {
    item_inner(content, true, true, true, style)
}

pub fn group_inner<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    first: bool,
    last: bool,
    style: Style,
) -> Element<'a, Message> {
    item_inner(content, first, last, false, style)
}

pub fn group<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    style: Style,
) -> Element<'a, Message> {
    // Paint translucent islands once: adjacent independently blended quads
    // can leave a visible seam even when their layout bounds touch.
    Element::new(Hover {
        radius: style.decoration.radius.into(),
        style,
        root: false,
        background: true,
        interactive: false,
        report: None,
        content: content.into(),
    })
}

fn item_inner<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    first: bool,
    last: bool,
    background: bool,
    style: Style,
) -> Element<'a, Message> {
    Element::new(Hover {
        radius: iced::border::Radius {
            top_left: if first { style.decoration.radius } else { 0.0 },
            bottom_left: if first { style.decoration.radius } else { 0.0 },
            top_right: if last { style.decoration.radius } else { 0.0 },
            bottom_right: if last { style.decoration.radius } else { 0.0 },
        },
        style,
        root: false,
        background,
        interactive: true,
        report: None,
        content: container(content)
            .padding([0.0, style.widgets.padding_horizontal])
            .center_y(style.widgets.height)
            .into(),
    })
}

pub fn decorated<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    style: Style,
    report: impl Fn(Vec<Decoration>) -> Message + 'a,
) -> Element<'a, Message> {
    Element::new(Hover {
        radius: style.decoration.radius.into(),
        style,
        root: true,
        background: true,
        interactive: false,
        report: style
            .decoration
            .blur
            .then(|| Box::new(report) as Box<dyn Fn(Vec<Decoration>) -> Message>),
        content: content.into(),
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
    style: Style,
    root: bool,
    background: bool,
    interactive: bool,
    report: Option<Box<dyn Fn(Vec<Decoration>) -> Message + 'a>>,
    content: Element<'a, Message>,
}

impl<Message> Widget<Message, Theme, Renderer> for Hover<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(State::default())
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
        if !self.root && self.background {
            operation.custom(None, layout.bounds(), &mut self.radius);
        }
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
        let hovered = self.interactive
            && cursor.is_over(layout.bounds())
            && cursor.is_over(*viewport);
        let state = tree.state.downcast_mut::<State>();
        if state.hovered != hovered {
            state.hovered = hovered;
            shell.request_redraw();
        }
        if let Some(report) = &self.report {
            let decorations = if self.style.decoration.layout
                == ghost_shell_config::BarLayout::Continuous
            {
                vec![Decoration {
                    bounds: layout.bounds(),
                    radius: self.radius,
                }]
            } else {
                let mut collector = Decorations(Vec::new());
                self.content.as_widget_mut().operate(
                    &mut tree.children[0],
                    layout,
                    renderer,
                    &mut collector,
                );
                collector.0
            };
            if state.decorations.as_ref() != Some(&decorations) {
                shell.publish(report(decorations.clone()));
                state.decorations = Some(decorations);
            }
        }
        if self.report.is_none() {
            state.decorations = None;
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
        let hovered = self.interactive
            && cursor.is_over(layout.bounds())
            && cursor.is_over(*viewport);
        let decorated = self.background
            && self.root
                == (self.style.decoration.layout
                    == ghost_shell_config::BarLayout::Continuous);
        let background = if hovered {
            Some(
                theme
                    .extended_palette()
                    .secondary
                    .strong
                    .color,
            )
        } else if decorated {
            Some(
                theme
                    .palette()
                    .background
                    .scale_alpha(self.style.decoration.opacity),
            )
        } else {
            None
        };
        if let Some(background) = background {
            container::draw_background(
                renderer,
                &container::Style {
                    // Keep individual hover edges aligned to physical pixels.
                    snap: true,
                    ..Default::default()
                }
                .background(background)
                .border(iced::Border {
                    radius: self.radius,
                    ..Default::default()
                }),
                layout.bounds(),
            );
        }
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

pub fn svg<'a, Message: 'a>(
    handle: iced::advanced::svg::Handle,
    size: f32,
) -> Element<'a, Message> {
    Element::new(Symbol(handle, size))
}

struct Symbol(iced::advanced::svg::Handle, f32);

impl<Message> Widget<Message, Theme, Renderer> for Symbol {
    fn size(&self) -> Size<Length> {
        Size::new(self.1.into(), self.1.into())
    }
    fn layout(
        &mut self,
        _: &mut Tree,
        _: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(self.1, self.1, Size::ZERO))
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

pub fn icon<'a, Message: 'a>(name: &str, size: f32) -> Element<'a, Message> {
    match ghost_shell_assets::Assets::get(name) {
        Some(asset) => svg(
            iced::advanced::svg::Handle::from_memory(asset.data.into_owned()),
            size,
        ),
        None => iced::widget::text("•").into(),
    }
}

struct Decorations(Vec<Decoration>);

impl Operation for Decorations {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }
    fn custom(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        state: &mut dyn std::any::Any,
    ) {
        if let Some(radius) = state.downcast_ref::<iced::border::Radius>() {
            self.0.push(Decoration {
                bounds,
                radius: *radius,
            });
        }
    }
}
