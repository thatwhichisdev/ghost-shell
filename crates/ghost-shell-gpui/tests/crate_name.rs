use ghost_shell_gpui::{
    Action as _, App, IntoElement as _, RenderOnce, TestAppContext, Window, actions, div,
};

actions!(crate_name_test, [Confirm]);

#[derive(ghost_shell_gpui::IntoElement)]
struct Component;

impl RenderOnce for Component {
    fn render(self, _: &mut Window, _: &mut App) -> impl ghost_shell_gpui::IntoElement {
        div()
    }
}

#[ghost_shell_gpui::test]
fn macros_work_without_a_gpui_alias(cx: &mut TestAppContext) {
    cx.update(|_| {
        assert!(Confirm.partial_eq(Confirm.boxed_clone().as_ref()));
        let _element = Component.into_element();
        let _text = ghost_shell_gpui::text!("Local GPUI");
    });
}
