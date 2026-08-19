//! Lockscreen presentation and input handling.
use std::{ffi::OsString, time::Duration};

use anyhow::Result;
use ghost_shell_wallpaper::wallpaper::{
    Wallpaper, WallpaperManager, WallpaperSource,
};
use gpui::{
    Context, Entity, FontWeight, IntoElement, Render, SharedString,
    Subscription, Task, Window, div, prelude::*, px, relative, rgb,
};
use gpui_component::input::{Input, InputContentType, InputEvent, InputState};
use jiff::Zoned;

use crate::{Authenticate, Unlock, auth};

/// Renders the lockscreen for a single display.
pub struct LockView {
    password: Entity<InputState>,

    _password_subscription: Subscription,

    clock: SharedString,

    _clock_update: Task<()>,

    authenticating: bool,

    wallpaper: Option<Entity<Wallpaper>>,
}

impl LockView {
    /// Creates a lockscreen view and starts its input and clock state.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            let input = InputState::new(window, cx).masked(true);
            input.focus(window, cx);
            input
        });

        let input_sub =
            cx.subscribe_in(&input, window, Self::handle_password_event);

        let source = cx.global::<WallpaperManager>().source();

        let wallpaper = source.map(|source| {
            cx.new(|cx| match source {
                WallpaperSource::Animated(animated) => {
                    Wallpaper::new(animated, window, cx).unwrap()
                }

                WallpaperSource::Static(_) => {
                    todo!("static wallpaper")
                }
            })
        });

        Self {
            password: input,
            clock: Self::formatted_time("%H:%M"),
            _password_subscription: input_sub,
            _clock_update: Self::spawn_clock_task(cx),
            authenticating: false,
            wallpaper,
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

    fn spawn_authentication_task(
        username: OsString,
        password: OsString,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        gpui_tokio::Tokio::spawn_result(cx, async move {
            auth::authenticate(&username, &password)
                .map_err(|e| anyhow::Error::msg(e.to_string()))
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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.authenticating {
            return;
        }

        let username: OsString = auth::username();
        let password: OsString =
            self.password.read(cx).value().to_string().into();

        // todo: add UI validations to now allow empty password
        if password.is_empty() || username.is_empty() {
            return;
        }

        self.authenticating = true;

        let authentication =
            Self::spawn_authentication_task(username, password, cx);

        cx.spawn_in(window, async move |view, cx| {
            let result = authentication.await;

            let _ = view.update_in(cx, |view, window, cx| {
                view.authenticating = false;

                match result {
                    Ok(()) => window.dispatch_action(Box::new(Unlock), cx),
                    Err(e) => log::error!("User authentication failed: {e}"),
                }
            });
        })
        .detach();
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
            .when_some(self.wallpaper.clone(), |this, wallpaper| {
                this.child(wallpaper)
            })
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
