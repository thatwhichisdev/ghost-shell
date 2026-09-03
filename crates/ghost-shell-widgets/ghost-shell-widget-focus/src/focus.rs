use ghost_shell_niri::NiriState;
use gpui::{Context, SharedString, Subscription, Window, div, prelude::*};

pub struct FocusWidget {
    pub title: SharedString,

    #[allow(unused)]
    subscription: Subscription,
}

impl FocusWidget {
    #[must_use]
    pub fn new(cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe_global::<NiriState>(|widget, cx| {
            let title: SharedString = cx
                .global::<NiriState>()
                .windows
                .values()
                .find(|window| window.is_focused)
                .and_then(|window| window.title.as_deref())
                .unwrap_or_default()
                .to_owned()
                .into();

            widget.title = title;

            cx.notify();
        });

        Self {
            title: SharedString::default(),
            subscription,
        }
    }
}

impl Render for FocusWidget {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("focus")
            .truncate()
            .child(self.title.clone())
    }
}
