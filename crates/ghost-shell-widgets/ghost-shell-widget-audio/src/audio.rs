use ghost_shell_audio::{
    Audio, AudioConnectionStatus, AudioDirection, AudioEndpoint, AudioEndpointId,
    AudioEvent,
};
use ghost_shell_component_icon::Icon;
use ghost_shell_component_root::Root;
use ghost_shell_component_slider::{Slider, SliderEvent, SliderState};
use ghost_shell_gpui::{
    AnyWindowHandle, Bounds, Context, Entity, IntoElement, Pixels, Point, Render,
    Subscription, Window, WindowBounds, WindowKind, WindowOptions, div, point,
    popup::{PopupAnchor, PopupConstraintAdjustment, PopupGravity, PopupOptions},
    prelude::*,
    px, size,
};
use ghost_shell_theme::{ActiveTheme, Sizable};

fn device_icon(direction: AudioDirection) -> Icon {
    let data: &[u8] = match direction {
        AudioDirection::Output => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4 5 9H2v6h3l6 5Z"/><path d="M15 8a6 6 0 0 1 0 8m3-11a10 10 0 0 1 0 14"/></svg>"#,
        AudioDirection::Input => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><rect x="9" y="2" width="6" height="13" rx="3"/><path d="M5 10v2a7 7 0 0 0 14 0v-2M12 19v3m-4 0h8"/></svg>"#,
    };
    Icon::default().data(data).with_size(px(18.))
}

fn device_name(endpoint: &AudioEndpoint) -> &str {
    if endpoint.description.is_empty() {
        &endpoint.name
    } else {
        &endpoint.description
    }
}

pub struct AudioWidget {
    audio: Entity<Audio>,
    popup: Option<AnyWindowHandle>,
    error: Option<String>,
    _subscription: Subscription,
}

impl AudioWidget {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let audio = ghost_shell_audio::init(cx);
        let subscription = cx.observe(&audio, |_, _, cx| cx.notify());
        Self {
            audio,
            popup: None,
            error: None,
            _subscription: subscription,
        }
    }

    fn toggle(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(popup) = self
            .popup
            .take()
            .filter(|popup| cx.windows().contains(popup))
        {
            if let Err(error) = popup.update(cx, |_, window, _| window.remove_window()) {
                self.error = Some(format!("Could not close audio controls: {error}"));
                cx.notify();
            }
            return;
        }
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                Default::default(),
                size(px(380.), px(460.)),
            ))),
            kind: WindowKind::AnchoredPopup(PopupOptions {
                parent: window.window_handle(),
                anchor_rect: Bounds::new(position, size(px(1.), px(1.))),
                anchor: PopupAnchor::BottomRight,
                gravity: PopupGravity::BottomLeft,
                constraint_adjustment: PopupConstraintAdjustment::SLIDE_X
                    | PopupConstraintAdjustment::SLIDE_Y
                    | PopupConstraintAdjustment::FLIP_X
                    | PopupConstraintAdjustment::FLIP_Y,
                offset: point(px(0.), px(8.)),
                grab: true,
            }),
            titlebar: None,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            ..Default::default()
        };
        let audio = self.audio.clone();
        match cx.open_window(options, move |window, cx| {
            let panel = cx.new(|cx| AudioPanel::new(audio, window, cx));
            cx.new(|cx| Root::new(panel, window, cx))
        }) {
            Ok(handle) => {
                self.popup = Some(handle.into());
                self.error = None;
            }
            Err(error) => {
                log::error!("Could not open audio controls: {error:#}");
                self.error = Some(format!("Could not open audio controls: {error}"));
            }
        }
        cx.notify();
    }
}

impl Render for AudioWidget {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.audio.read(cx).state();
        let output = state
            .default_output()
            .and_then(|id| state.endpoints().get(&id));
        let label = output
            .map(|endpoint| {
                if endpoint.muted == Some(true) {
                    "Muted".to_owned()
                } else {
                    endpoint
                        .volume
                        .map(|volume| format!("{:.0}%", volume * 100.))
                        .unwrap_or_else(|| "—".into())
                }
            })
            .unwrap_or_else(|| "—".into());
        div()
            .id("audio-widget")
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .cursor_pointer()
            .aria_label("Open audio controls")
            .child(device_icon(AudioDirection::Output).text_color(cx.theme().foreground))
            .child(label)
            .when_some(self.error.clone(), |element, error| {
                element.child(
                    div()
                        .text_color(cx.theme().destructive)
                        .child(error),
                )
            })
            .on_click(cx.listener(
                |this, event: &ghost_shell_gpui::ClickEvent, window, cx| {
                    this.toggle(event.position(), window, cx)
                },
            ))
    }
}

struct VolumeControl {
    slider: Entity<SliderState>,
    endpoint: Option<AudioEndpointId>,
    editing: bool,
}

pub struct AudioPanel {
    focus_handle: ghost_shell_gpui::FocusHandle,
    audio: Entity<Audio>,
    output: VolumeControl,
    input: VolumeControl,
    category: AudioDirection,
    error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl AudioPanel {
    pub fn new(
        audio: Entity<Audio>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let output = cx.new(|_| SliderState::new());
        let input = cx.new(|_| SliderState::new());
        let subscriptions = vec![
            cx.observe_in(&audio, window, |this, _, window, cx| {
                this.sync(window, cx);
                cx.notify();
            }),
            cx.subscribe(&audio, |this, _, event, cx| {
                if let AudioEvent::CommandFailed(error) = event {
                    this.error = Some(error.message.clone());
                    cx.notify();
                }
            }),
            cx.subscribe_in(&output, window, |this, _, event, window, cx| {
                this.change_volume(AudioDirection::Output, event, window, cx)
            }),
            cx.subscribe_in(&input, window, |this, _, event, window, cx| {
                this.change_volume(AudioDirection::Input, event, window, cx)
            }),
        ];
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);
        let mut panel = Self {
            focus_handle,
            audio,
            output: VolumeControl {
                slider: output,
                endpoint: None,
                editing: false,
            },
            input: VolumeControl {
                slider: input,
                endpoint: None,
                editing: false,
            },
            category: AudioDirection::Output,
            error: None,
            _subscriptions: subscriptions,
        };
        panel.sync(window, cx);
        panel
    }

    fn sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = self.audio.read(cx).state();
        let values = [state.default_output(), state.default_input()].map(|endpoint| {
            let value = endpoint
                .and_then(|id| state.endpoints().get(&id))
                .and_then(|endpoint| endpoint.volume)
                .unwrap_or(0.)
                * 100.;
            (endpoint, value)
        });
        for (control, (endpoint, value)) in [&mut self.output, &mut self.input]
            .into_iter()
            .zip(values)
        {
            // Preserve the gesture's endpoint and position while backend echoes arrive.
            if control.editing {
                continue;
            }
            control.endpoint = endpoint;
            if control.slider.read(cx).value().end() != value.clamp(0., 100.) {
                control
                    .slider
                    .update(cx, |slider, cx| slider.set_value(value, window, cx));
            }
        }
    }

    fn change_volume(
        &mut self,
        direction: AudioDirection,
        event: &SliderEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let control = match direction {
            AudioDirection::Output => &mut self.output,
            AudioDirection::Input => &mut self.input,
        };
        let value = match event {
            SliderEvent::Change(value) => {
                control.editing = true;
                *value
            }
            SliderEvent::Release(value) => {
                control.editing = false;
                *value
            }
        };
        let endpoint = control.endpoint;
        if let Some(endpoint) = endpoint {
            self.error = self
                .audio
                .read(cx)
                .set_volume(endpoint, (value.end() / 100.).clamp(0., 1.))
                .err()
                .map(|error| error.to_string());
        }
        if matches!(event, SliderEvent::Release(_)) {
            let state = self.audio.read(cx).state();
            let current = match direction {
                AudioDirection::Output => state.default_output(),
                AudioDirection::Input => state.default_input(),
            };
            if current != endpoint || self.error.is_some() {
                self.sync(window, cx);
            }
        }
        cx.notify();
    }

    fn select_device(&mut self, endpoint: AudioEndpointId, cx: &mut Context<Self>) {
        self.error = self
            .audio
            .read(cx)
            .set_default(endpoint)
            .err()
            .map(|error| error.to_string());
        cx.notify();
    }

    fn control(
        &self,
        direction: AudioDirection,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let control = match direction {
            AudioDirection::Output => &self.output,
            AudioDirection::Input => &self.input,
        };
        let state = self.audio.read(cx).state();
        let endpoint = control
            .endpoint
            .and_then(|id| state.endpoints().get(&id));
        let title = match direction {
            AudioDirection::Output => "Output",
            AudioDirection::Input => "Input",
        };
        let name = endpoint
            .map(device_name)
            .unwrap_or(match direction {
                AudioDirection::Output => "No default output device",
                AudioDirection::Input => "No default input device",
            })
            .to_owned();
        // Keep the gesture alive through a disconnect so release can reconcile its target.
        let enabled = control.editing
            || (endpoint.is_some_and(|endpoint| endpoint.volume.is_some())
                && state.connection_status() == &AudioConnectionStatus::Connected);
        let volume = if enabled {
            format!("{:.0}%", control.slider.read(cx).value().end())
        } else {
            "—".into()
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded_lg()
            .bg(cx.theme().secondary)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(device_icon(direction).text_color(cx.theme().primary))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(title),
                            )
                            .child(div().truncate().child(name)),
                    )
                    .child(div().text_xs().child(volume))
                    .when(
                        endpoint.is_some_and(|endpoint| endpoint.muted == Some(true)),
                        |row| row.child(div().text_xs().child("Muted")),
                    ),
            )
            .child(
                div()
                    .px_2()
                    .child(Slider::new(&control.slider).disabled(!enabled)),
            )
    }
}

impl Render for AudioPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.audio.read(cx).state();
        let status = match state.connection_status() {
            AudioConnectionStatus::Connecting => Some("Connecting to audio…".to_owned()),
            AudioConnectionStatus::Unavailable(message) => {
                Some(format!("Audio unavailable: {message}"))
            }
            AudioConnectionStatus::Stopped => Some("Audio service stopped".to_owned()),
            AudioConnectionStatus::Connected => None,
        };
        let default = match self.category {
            AudioDirection::Output => state.default_output(),
            AudioDirection::Input => state.default_input(),
        };
        let rows: Vec<_> = state
            .endpoints()
            .values()
            .filter(|endpoint| endpoint.direction == self.category)
            .enumerate()
            .map(|(index, endpoint)| {
                let endpoint_id = endpoint.id;
                let selectable = state.connection_status()
                    == &AudioConnectionStatus::Connected
                    && Some(endpoint_id) != default;
                div()
                    .id(("audio-device", index))
                    .role(ghost_shell_gpui::Role::Button)
                    .aria_label(format!("Use {} as default", device_name(endpoint)))
                    .when(selectable, |row| {
                        row.tab_index(0)
                            .cursor_pointer()
                            .hover(|style| style.bg(cx.theme().secondary))
                            .focus(|style| style.bg(cx.theme().secondary))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_device(endpoint_id, cx)
                            }))
                            .on_key_down(cx.listener(
                                move |this,
                                      event: &ghost_shell_gpui::KeyDownEvent,
                                      _,
                                      cx| {
                                    if matches!(
                                        event.keystroke.key.as_str(),
                                        "enter" | "space"
                                    ) {
                                        this.select_device(endpoint_id, cx);
                                        cx.stop_propagation();
                                    }
                                },
                            ))
                    })
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        device_icon(endpoint.direction)
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(
                        div().flex_1().min_w_0().child(
                            div()
                                .truncate()
                                .child(device_name(endpoint).to_owned()),
                        ),
                    )
                    .when(Some(endpoint.id) == default, |row| {
                        row.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().primary)
                                .child("Default"),
                        )
                    })
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(if endpoint.muted == Some(true) {
                                "Muted".into()
                            } else {
                                endpoint
                                    .volume
                                    .map(|volume| format!("{:.0}%", volume * 100.))
                                    .unwrap_or_else(|| "—".into())
                            }),
                    )
            })
            .collect();
        let empty = rows.is_empty();
        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .text_sm()
            .on_key_down(|event, window, cx| {
                if event.keystroke.key == "escape" {
                    window.remove_window();
                    cx.stop_propagation();
                }
            })
            .child(self.control(AudioDirection::Output, cx))
            .child(self.control(AudioDirection::Input, cx))
            .child(
                div().flex().gap_2().children(
                    [AudioDirection::Output, AudioDirection::Input]
                        .into_iter()
                        .map(|category| {
                            let selected = self.category == category;
                            let label = match category {
                                AudioDirection::Output => "Outputs",
                                AudioDirection::Input => "Inputs",
                            };
                            div()
                                .id(label)
                                .tab_index(0)
                                .flex_1()
                                .flex()
                                .justify_center()
                                .gap_2()
                                .py_2()
                                .rounded_md()
                                .cursor_pointer()
                                .bg(if selected {
                                    cx.theme().primary
                                } else {
                                    cx.theme().secondary
                                })
                                .text_color(if selected {
                                    cx.theme().primary_foreground
                                } else {
                                    cx.theme().foreground
                                })
                                .child(device_icon(category))
                                .child(label)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.category = category;
                                    cx.notify();
                                }))
                                .on_key_down(cx.listener(
                                    move |this,
                                          event: &ghost_shell_gpui::KeyDownEvent,
                                          _,
                                          cx| {
                                        if matches!(
                                            event.keystroke.key.as_str(),
                                            "enter" | "space"
                                        ) {
                                            this.category = category;
                                            cx.notify();
                                            cx.stop_propagation();
                                        }
                                    },
                                ))
                        }),
                ),
            )
            .when_some(status, |panel, status| {
                panel.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(status),
                )
            })
            .when_some(self.error.clone(), |panel, error| {
                panel.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().destructive)
                        .child(error),
                )
            })
            .child(
                div()
                    .id("audio-devices")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_md()
                    .when(empty, |list| {
                        list.child(
                            div()
                                .p_3()
                                .text_color(cx.theme().muted_foreground)
                                .child("No devices available"),
                        )
                    })
                    .children(rows),
            )
    }
}
