// Adapted from GPUI Kit 0e63ea799766, copyright 2024 - 2026 Longbridge.
// Modified for Ghost Shell; see LICENSE.md, NOTICE, and crates/ghost-shell-components/UPSTREAM.md.

use ghost_shell_gpui::{Pixels, px};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Size {
    Size(Pixels),
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
}

impl Size {
    pub fn input_px(self) -> Pixels {
        match self {
            Self::XSmall => px(4.),
            Self::Small | Self::Size(_) => px(8.),
            Self::Medium => px(10.),
            Self::Large => px(12.),
        }
    }

    pub fn input_py(self) -> Pixels {
        match self {
            Self::XSmall => px(0.),
            Self::Small | Self::Size(_) => px(2.),
            Self::Medium => px(8.),
            Self::Large => px(10.),
        }
    }

    pub fn row_height(self) -> Pixels {
        match self {
            Self::Size(size) => size,
            Self::XSmall => px(26.),
            Self::Small => px(30.),
            Self::Medium => px(32.),
            Self::Large => px(40.),
        }
    }
}

impl From<Pixels> for Size {
    fn from(size: Pixels) -> Self {
        Self::Size(size)
    }
}

pub trait Sizable: Sized {
    fn with_size(self, size: impl Into<Size>) -> Self;

    fn xsmall(self) -> Self {
        self.with_size(Size::XSmall)
    }

    fn small(self) -> Self {
        self.with_size(Size::Small)
    }

    fn large(self) -> Self {
        self.with_size(Size::Large)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizing_preserves_upstream_metrics() {
        assert_eq!(Size::XSmall.row_height(), px(26.));
        assert_eq!(Size::Medium.row_height(), px(32.));
        assert_eq!(Size::from(px(48.)).row_height(), px(48.));
        assert_eq!(Size::Small.input_px(), px(8.));
        assert_eq!(Size::Large.input_py(), px(10.));
    }
}
