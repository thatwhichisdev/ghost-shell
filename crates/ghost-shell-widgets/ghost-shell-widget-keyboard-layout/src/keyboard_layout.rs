use ghost_shell_niri::NiriState;
use iced::{Element, widget::text};

#[derive(Default)]
pub struct KeyboardLayout {
    label: Option<&'static str>,
}

impl KeyboardLayout {
    pub fn update(&mut self, niri: &NiriState) {
        self.label = label(niri);
    }

    pub fn view<'a, Message: 'a>(
        &self,
        style: ghost_shell_theme::bar::Style,
    ) -> Element<'a, Message> {
        let Some(label) = self.label else {
            return iced::widget::Space::new()
                .width(0)
                .height(0)
                .into();
        };

        ghost_shell_theme::bar::outer(
            ghost_shell_theme::bar::inner(
                text(label)
                    .font(style.font)
                    .size(style.font_size),
                style,
            ),
            style,
        )
    }
}

fn label(niri: &NiriState) -> Option<&'static str> {
    let layouts = niri.keyboard_layouts.as_ref()?;
    let name = layouts
        .names
        .get(usize::from(layouts.current_idx))?;
    let language = name.split('(').next()?.trim();
    if language.is_empty() {
        return None;
    }
    Some(
        isolang::Language::from_name(language)
            .and_then(|language| language.to_639_1())
            .unwrap_or("--"),
    )
}

#[cfg(test)]
mod tests {
    use ghost_shell_niri::{KeyboardLayouts, NiriEvent};

    use super::*;

    #[test]
    fn follows_initial_layout_switches_and_replacement() {
        let mut niri = NiriState::default();
        let mut widget = KeyboardLayout::default();
        widget.update(&niri);
        assert_eq!(widget.label, None);

        niri.update(NiriEvent::KeyboardLayoutsChanged {
            keyboard_layouts: KeyboardLayouts {
                names: vec!["English (US)".into(), "Russian".into()],
                current_idx: 1,
            },
        });
        widget.update(&niri);
        assert_eq!(widget.label, Some("ru"));

        niri.update(NiriEvent::KeyboardLayoutSwitched { idx: 0 });
        widget.update(&niri);
        assert_eq!(widget.label, Some("en"));

        niri.update(NiriEvent::KeyboardLayoutsChanged {
            keyboard_layouts: KeyboardLayouts {
                names: vec!["French".into()],
                current_idx: 0,
            },
        });
        widget.update(&niri);
        assert_eq!(widget.label, Some("fr"));

        niri.update(NiriEvent::KeyboardLayoutSwitched { idx: 255 });
        widget.update(&niri);
        assert_eq!(widget.label, None);
        niri = NiriState::default();
        widget.update(&niri);
        assert_eq!(widget.label, None);
    }

    #[test]
    fn handles_missing_and_empty_names_without_panicking() {
        for names in [vec![], vec![String::new()]] {
            let niri = NiriState {
                keyboard_layouts: Some(KeyboardLayouts {
                    names,
                    current_idx: 0,
                }),
                ..Default::default()
            };
            assert_eq!(label(&niri), None);
        }
    }

    #[test]
    fn uses_language_codes_and_handles_unknown_layouts() {
        for (name, expected) in [
            ("German", "de"),
            ("Spanish (Latin American)", "es"),
            ("Ukrainian", "uk"),
            ("Japanese", "ja"),
            ("English (US, intl., with dead keys)", "en"),
            ("  French (Canada)  ", "fr"),
            ("Custom layout", "--"),
            ("ÉCOLE", "--"),
            ("A", "--"),
        ] {
            let niri = NiriState {
                keyboard_layouts: Some(KeyboardLayouts {
                    names: vec![name.into()],
                    current_idx: 0,
                }),
                ..Default::default()
            };
            assert_eq!(label(&niri), Some(expected), "{name}");
        }
    }
}
