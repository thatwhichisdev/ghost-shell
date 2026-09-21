use ghost_shell_gpui::{
    Context, Subscription, Window, accesskit::Uuid, div, prelude::*, px, svg,
};
use ghost_shell_niri::NiriState;
use ghost_shell_theme::ActiveTheme as _;

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
            widget.state =
                Self::workspaces(cx.global::<NiriState>(), widget.display_uuid);
            cx.notify();
        });

        Self {
            display_uuid,
            state: Self::workspaces(cx.global::<NiriState>(), display_uuid),
            subscription,
        }
    }

    fn workspaces(state: &NiriState, display_uuid: Uuid) -> Vec<Workspace> {
        let mut workspaces: Vec<_> = state
            .workspaces
            .values()
            .filter(|workspace| {
                workspace
                    .output
                    .as_ref()
                    .is_some_and(|output| {
                        Uuid::new_v5(&Uuid::NAMESPACE_DNS, output.as_bytes())
                            == display_uuid
                    })
            })
            .map(|workspace| Workspace {
                idx: workspace.idx,
                is_active: workspace.is_active,
            })
            .collect();
        workspaces.sort_by_key(|workspace| workspace.idx);
        workspaces
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
                        .text_color(cx.theme().foreground)
                } else {
                    svg()
                        .path("icons/circle.svg")
                        .size(px(18.0))
                        .text_color(cx.theme().foreground)
                }
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspaces_without_an_output_are_ignored() {
        let state = NiriState {
            workspaces: [(
                1,
                ghost_shell_niri::Workspace {
                    id: 1,
                    idx: 1,
                    name: None,
                    output: None,
                    is_urgent: false,
                    is_active: true,
                    is_focused: true,
                    active_window_id: None,
                },
            )]
            .into(),
            ..Default::default()
        };
        let display_uuid = Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"DP-1");
        assert!(WorkspacesWidget::workspaces(&state, display_uuid).is_empty());
    }
}
