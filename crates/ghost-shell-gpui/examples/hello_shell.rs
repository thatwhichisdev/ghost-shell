use ghost_shell_gpui::{
    AppContext, Context, IntoElement, Render, Window, WindowOptions, div, prelude::*,
};

struct HelloShell;

impl Render for HelloShell {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(ghost_shell_gpui::rgb(0x181818))
            .text_color(ghost_shell_gpui::rgb(0xeeeeee))
            .child("Ghost GPUI — Wayland")
    }
}

fn main() {
    ghost_shell_gpui::application().run(|cx| {
        if let Err(error) =
            cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| HelloShell))
        {
            eprintln!("Failed to open the GPUI example: {error:#}");
            cx.quit();
        }
    });
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use ghost_shell_gpui::{
        Action as _, App, IntoElement as _, IsEmpty as _, Refineable, RenderOnce,
        TestAppContext, Window, actions, div,
    };

    actions!(crate_name_test, [Confirm]);

    #[derive(ghost_shell_gpui::IntoElement)]
    struct Component;

    impl RenderOnce for Component {
        fn render(
            self,
            _: &mut Window,
            _: &mut App,
        ) -> impl ghost_shell_gpui::IntoElement {
            div()
        }
    }

    #[derive(Clone, Default, Refineable)]
    struct Appearance {
        opacity: f32,
    }

    #[ghost_shell_gpui::test]
    fn macros_work_through_the_public_crate(cx: &mut TestAppContext) {
        cx.update(|_| {
            assert!(Confirm.partial_eq(Confirm.boxed_clone().as_ref()));
            let _element = Component.into_element();
            let _text = ghost_shell_gpui::text!("Local GPUI");
            assert!(AppearanceRefinement::default().is_empty());
            let appearance = Appearance::default()
                .refined(AppearanceRefinement { opacity: Some(1.0) });
            assert_eq!(appearance.opacity, 1.0);
        });
    }
}
