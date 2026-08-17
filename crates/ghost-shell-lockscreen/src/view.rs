use std::{ffi::OsStr, time::Duration};

use gpui::{
    App, Context, Entity, FocusHandle, Focusable, FontWeight, IntoElement,
    Render, Task, Window, div, prelude::*, px, relative, rgb,
};
use gpui_component::input::{Input, InputContentType, InputEvent, InputState};
use jiff::Zoned;

use crate::{Authenticate, Unlock, auth};

pub struct View {
    input_password: Entity<InputState>,
    hours: String,
    minutes: String,
    _clock_task: Task<()>,
}

impl View {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let now = Zoned::now();

        let clock_task = cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(
                        60 - Zoned::now().second() as u64,
                    ))
                    .await;

                if view
                    .update(cx, |this, cx| {
                        let now = Zoned::now();
                        this.hours = format!("{:02}", now.hour());
                        this.minutes = format!("{:02}", now.minute());
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        });

        let input_password = cx.new(|cx| {
            let input = InputState::new(window, cx).masked(true);
            input.focus(window, cx);
            input
        });

        cx.subscribe_in(
            &input_password,
            window,
            |_this, _state, event: &InputEvent, window, cx| {
                if matches!(
                    event,
                    InputEvent::PressEnter {
                        secondary: _,
                        shift: _
                    }
                ) {
                    window.dispatch_action(Box::new(Authenticate), cx);
                }
            },
        )
        .detach();

        Self {
            input_password,
            hours: format!("{:02}", now.hour()),
            minutes: format!("{:02}", now.minute()),
            _clock_task: clock_task,
        }
    }

    fn authenticate(
        &mut self,
        _: &Authenticate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let password = self.input_password.read(cx).value().to_string();

        if password.is_empty() {
            return;
        }

        let username = std::env::var_os("USER")
            .filter(|username| !username.is_empty())
            .unwrap();

        match auth::authenticate(&username, OsStr::new(&password)) {
            Ok(()) => window.dispatch_action(Box::new(Unlock), cx),
            Err(err) => {
                eprintln!("Failed to authenticate user, error code {err:#}")
            }
        }
    }
}

impl Render for View {
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
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(
                                div()
                                    .w_full()
                                    .text_center()
                                    .child(self.hours.clone()),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .text_center()
                                    .child(self.minutes.clone()),
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
                        Input::new(&self.input_password)
                            .appearance(false)
                            .content_type(InputContentType::Password)
                            .w(px(240.0))
                            .text_center()
                            .text_size(px(28.0)),
                    ),
            )
    }
}
