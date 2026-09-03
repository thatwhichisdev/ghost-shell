use std::borrow::Cow;

use anyhow::anyhow;
use gpui::{AssetSource, Result, SharedString};

#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "**/*.svg"]
pub struct Assets;

impl Assets {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for Assets {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow!("could not find asset at path \"{path}\""))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}
