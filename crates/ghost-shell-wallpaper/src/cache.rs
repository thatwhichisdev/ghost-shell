use std::{
    ffi::OsStr,
    fs::File,
    io::{BufWriter, Write as _},
    path::PathBuf,
};

use anyhow::{Context as _, Result};
use ghost_shell_config::config;
use serde::{Serialize, de::DeserializeOwned};

pub fn save<T>(name: &OsStr, data: &T) -> Result<PathBuf>
where
    T: Serialize + ?Sized,
{
    let cache_path = config::cache_dir()?.join(name);

    let file = File::create(&cache_path).with_context(|| {
        format!("failed to create wallpaper cache {}", cache_path.display())
    })?;

    let writer = BufWriter::new(file);

    let mut writer = postcard::to_io(data, writer).with_context(|| {
        format!(
            "failed to serialize cache file at path: {}",
            cache_path.display()
        )
    })?;

    writer.flush().with_context(|| {
        format!(
            "failed to flush cache file at path: {}",
            cache_path.display()
        )
    })?;

    log::debug!("Cached file at {}", cache_path.display());

    Ok(cache_path)
}

pub fn load<T>(name: &OsStr) -> Result<Option<T>>
where
    T: DeserializeOwned,
{
    let cache_path = config::cache_dir()?.join(name);

    if !cache_path.exists() {
        return Ok(None);
    }

    let bytes = std::fs::read(&cache_path).with_context(|| {
        format!(
            "failed to read cache file at path: {}",
            cache_path.display()
        )
    })?;

    let data = postcard::from_bytes(&bytes).with_context(|| {
        format!(
            "failed to deserialize cache file at path: {} ",
            cache_path.display()
        )
    })?;

    log::debug!("Loaded file from cache {}", cache_path.display());

    Ok(Some(data))
}
