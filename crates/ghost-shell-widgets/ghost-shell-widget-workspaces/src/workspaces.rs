use ghost_shell_niri::NiriState;
use gpui::{
    Context, Subscription, Window, accesskit::Uuid, div, prelude::*, px, svg,
};

use gpui_component::ActiveTheme as _;

pub struct WorkspacesWidget {
    display_uuid: Uuid,

    state: Vec<Workspace>,

    #[allow(unused)]
    subscription: Subscription,
}

struct Workspace {
    idx: u8,
    is_active: bool,
}

impl WorkspacesWidget {
    #[must_use]
    pub fn new(cx: &mut Context<Self>, display_uuid: Uuid) -> Self {
        let subscription = cx.observe_global::<NiriState>(|widget, cx| {
            let mut state: Vec<Workspace> = cx
                .global::<NiriState>()
                .workspaces
                .values()
                .filter(|workspace| {
                    let output_name = workspace.output.as_ref().unwrap();
                    let output_uuid = Uuid::new_v5(
                        &Uuid::NAMESPACE_DNS,
                        output_name.as_bytes(),
                    );

                    output_uuid == widget.display_uuid
                })
                .map(|workspace| Workspace {
                    idx: workspace.idx,
                    is_active: workspace.is_active,
                })
                .collect();

            state.sort_by_key(|workspace| workspace.idx);

            widget.state = state;

            cx.notify();
        });

        Self {
            display_uuid,
            state: Vec::default(),
            subscription,
        }
    }
}

impl Render for WorkspacesWidget {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
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
                        .path("icons/circle-filled.svg")
                        .size(px(18.0))
                        .text_color(cx.theme().colors.foreground)
                } else {
                    svg()
                        .path("icons/circle.svg")
                        .size(px(18.0))
                        .text_color(cx.theme().colors.foreground)
                }
            }))
    }
}
