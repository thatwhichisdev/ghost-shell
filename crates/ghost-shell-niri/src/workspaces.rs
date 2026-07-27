use gpui::{Context, Window, div, prelude::*, px, rgba, svg};
use tokio::sync::broadcast;

use crate::client::state::NiriState;

pub struct Workspaces {
    output: String,
    state: Vec<Workspace>,
}

struct Workspace {
    idx: u8,
    is_active: bool,
}

impl Workspaces {
    #[must_use]
    pub fn new(
        cx: &mut Context<Self>,
        output: String,
        mut receiver: broadcast::Receiver<NiriState>,
    ) -> Self {
        // Spawn a task that will update workspaces when compositor's state has changed
        cx.spawn(async move |workspaces, cx| {
            loop {
                if let Ok(state) = receiver.recv().await {
                    workspaces
                        .update(cx, |workspaces, cx| {
                            let mut state: Vec<Workspace> = state
                                .workspaces
                                .values()
                                .filter(|workspace| {
                                    workspace.output.as_ref().is_some_and(
                                        |output| *output == workspaces.output,
                                    )
                                })
                                .map(|workspace| Workspace {
                                    idx: workspace.idx,
                                    is_active: workspace.is_active,
                                })
                                .collect();

                            state.sort_by_key(|workspace| workspace.idx);

                            workspaces.state = state;

                            cx.notify();
                        })
                        .unwrap();
                }
            }
        })
        .detach();

        Self {
            output,
            state: Vec::default(),
        }
    }
}

impl Render for Workspaces {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("workspaces")
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .children(self.state.iter().map(|workspace| {
                if workspace.is_active {
                    svg()
                        .path("circle-filled.svg")
                        .size(px(18.0))
                        .text_color(rgba(0xffff_ffff))
                } else {
                    svg()
                        .path("circle.svg")
                        .size(px(18.0))
                        .text_color(rgba(0xffff_ffff))
                }
            }))
    }
}
