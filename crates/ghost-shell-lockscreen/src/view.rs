use std::{ffi::OsStr, time::Duration};

use gpui::{
    Context, Entity, FontWeight, IntoElement, Render, SharedString,
    Subscription, Task, Window, div, prelude::*, px, relative, rgb,
};
use gpui_component::input::{Input, InputContentType, InputEvent, InputState};
use jiff::Zoned;

use crate::{Authenticate, auth};

pub struct LockView {
    password: Entity<InputState>,

    #[allow(unused)]
    password_subscription: Subscription,

    clock: SharedString,

    #[allow(unused)]
    clock_update: Task<()>,
}

impl LockView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            let input = InputState::new(window, cx).masked(true);
            input.focus(window, cx);
            input
        });

        let input_sub =
            cx.subscribe_in(&input, window, Self::handle_password_event);

        Self {
            password: input,
            clock: Self::formatted_time("%H:%M"),
            password_subscription: input_sub,
            clock_update: Self::spawn_clock_task(cx),
        }
    }

    fn formatted_time(format: &str) -> SharedString {
        Zoned::now().strftime(format).to_string().into()
    }

    fn spawn_clock_task(cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor().timer(Duration::from_mins(1)).await;

                if let Err(err) = view.update(cx, |view, cx| {
                    view.clock = Self::formatted_time("%H:%M");
                    cx.notify();
                }) {
                    log::debug!("Failed to update lockscreen's clock: {err}");
                }
            }
        })
    }

    fn handle_password_event(
        _view: &mut LockView,
        _state: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(
            event,
            InputEvent::PressEnter {
                secondary: _,
                shift: _
            }
        ) {
            window.dispatch_action(Box::new(Authenticate), cx);
        }
    }

    fn authenticate(
        &mut self,
        _: &Authenticate,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let username = auth::username();
        let password = self.password.read(cx).value().to_string();

        // todo: add UI validations to now allow empty password
        if password.is_empty() || username.is_empty() {
            return;
        }

        match auth::authenticate(&username, OsStr::new(&password)) {
            Ok(()) => match cx.unlock_session() {
                Ok(()) => log::debug!("Session unlocked"),
                Err(e) => log::error!("Session unlocking failed: {e:#}"),
            },
            Err(e) => {
                log::error!("User authentication failed: {e:#}")
            }
        }
    }
}

impl Render for LockView {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("lockscreen")
            .key_context("lockscreen")
            .on_action(cx.listener(Self::authenticate))
            .relative()
            .size_full()
            .text_color(rgb(0xffffff))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .line_height(relative(0.85))
                            .text_size(px(256.0))
                            .font_weight(FontWeight::BLACK)
                            .child(
                                div()
                                    .w_full()
                                    .text_center()
                                    .child(self.clock.clone()),
                            ),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .bottom(px(200.0))
                    .w_full()
                    .flex()
                    .justify_center()
                    .child(
                        Input::new(&self.password)
                            .appearance(false)
                            .content_type(InputContentType::Password)
                            .w(px(240.0))
                            .text_center()
                            .text_size(px(28.0)),
                    ),
            )
    }
}
