// Adapted from GPUI Kit 0e63ea799766, copyright 2024 - 2026 Longbridge.
// Modified for Ghost Shell; see LICENSE.md, NOTICE, and crates/ghost-shell-components/UPSTREAM.md.

mod focus;
mod sizing;
mod tokens;

#[cfg(feature = "legacy")]
pub mod legacy;

use std::ops::{Deref, DerefMut};

pub use focus::{FocusTrapContainer, FocusTrapElement, active_focus_trap};
use ghost_shell_gpui::{App, Global, WindowAppearance};
use serde::{Deserialize, Serialize};
pub use sizing::{Sizable, Size};
pub use tokens::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl From<WindowAppearance> for ThemeMode {
    fn from(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::Light,
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::Dark,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub mode: ThemeMode,
    pub tokens: SemanticThemeTokens,
    pub focus_ring: bool,
}

impl Global for Theme {}

impl Default for Theme {
    fn default() -> Self {
        Self::new(ThemeMode::Light)
    }
}

impl Theme {
    pub fn new(mode: ThemeMode) -> Self {
        let colors = match mode {
            ThemeMode::Light => ColorTokens::light(),
            ThemeMode::Dark => ColorTokens::dark(),
        };
        Self {
            mode,
            tokens: SemanticThemeTokens {
                colors,
                ..Default::default()
            },
            focus_ring: true,
        }
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn set(theme: Self, cx: &mut App) {
        cx.set_global(theme);
        cx.refresh_windows();
    }

    // Accept palette values instead of AppConfig: config still uses upstream GPUI
    // until the application migration, and must not enter the component graph.
    pub fn apply_base16(&mut self, palette: &[u32; 16]) {
        let color = |index: usize| {
            ghost_shell_gpui::Hsla::from(ghost_shell_gpui::rgb(palette[index]))
        };
        self.tokens.colors = ColorTokens {
            background: color(0),
            foreground: color(5),
            surface: color(0),
            surface_foreground: color(5),
            primary: color(13),
            primary_foreground: color(0),
            secondary: color(1),
            secondary_foreground: color(5),
            muted: color(1),
            muted_foreground: color(3),
            accent: color(1),
            accent_foreground: color(5),
            destructive: color(8),
            destructive_foreground: color(0),
            border: color(2),
            input: color(1),
            ring: color(13),
            selection: color(2),
        };
    }
}

impl Deref for Theme {
    type Target = ColorTokens;

    fn deref(&self) -> &Self::Target {
        &self.tokens.colors
    }
}

impl DerefMut for Theme {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tokens.colors
    }
}

pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    fn theme(&self) -> &Theme {
        Theme::global(self)
    }
}

pub fn init(cx: &mut App) {
    if !cx.has_global::<Theme>() {
        cx.set_global(Theme::new(cx.window_appearance().into()));
    }
    focus::init(cx);
}

#[cfg(test)]
mod tests {
    use ghost_shell_gpui::TestAppContext;

    use super::*;

    #[test]
    fn palettes_and_appearance_are_consistent() {
        assert_eq!(
            Theme::new(ThemeMode::Light).tokens.colors,
            ColorTokens::light()
        );
        assert_eq!(
            Theme::new(ThemeMode::Dark).tokens.colors,
            ColorTokens::dark()
        );
        assert_eq!(ThemeMode::from(WindowAppearance::Dark), ThemeMode::Dark);
    }

    #[test]
    fn base16_updates_the_single_source_of_colors() {
        let mut theme = Theme::new(ThemeMode::Dark);
        let palette = std::array::from_fn(|index| (index as u32) * 0x101010);
        theme.apply_base16(&palette);
        assert_eq!(
            theme.background,
            ghost_shell_gpui::Hsla::from(ghost_shell_gpui::rgb(palette[0]))
        );
        assert_eq!(
            theme.foreground,
            ghost_shell_gpui::Hsla::from(ghost_shell_gpui::rgb(palette[5]))
        );
        assert_eq!(
            theme.primary,
            ghost_shell_gpui::Hsla::from(ghost_shell_gpui::rgb(palette[13]))
        );
        assert_eq!(theme.mode, ThemeMode::Dark);
    }

    #[ghost_shell_gpui::test]
    fn initialization_preserves_the_installed_theme(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut theme = Theme::new(ThemeMode::Dark);
            theme.tokens.typography.sans = "Ghost Test".into();
            Theme::set(theme.clone(), cx);
            init(cx);
            init(cx);
            assert_eq!(cx.theme(), &theme);
        });
    }
}
