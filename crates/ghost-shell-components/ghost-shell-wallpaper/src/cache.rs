use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::Animation;

const VERSION: u32 = 1;
#[derive(Serialize, Deserialize, PartialEq)]
struct Identity {
    version: u32,
    path: PathBuf,
    length: u64,
    modified_nanos: u128,
}
#[derive(Serialize, Deserialize)]
struct Cached {
    identity: Identity,
    animation: Animation,
}
fn identity(path: &Path) -> Result<Identity> {
    let path = path.canonicalize()?;
    let metadata = path.metadata()?;
    Ok(Identity {
        version: VERSION,
        path,
        length: metadata.len(),
        modified_nanos: metadata
            .modified()?
            .duration_since(UNIX_EPOCH)?
            .as_nanos(),
    })
}
fn cache_path(path: &Path) -> Result<PathBuf> {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    path.canonicalize()?.hash(&mut hash);
    Ok(ghost_shell_config::cache_dir()?
        .join(format!("iced-wallpaper-{:016x}.bin", hash.finish())))
}
pub fn load(path: &Path) -> Result<Option<Animation>> {
    read(path, &cache_path(path)?)
}
fn read(path: &Path, cached: &Path) -> Result<Option<Animation>> {
    let identity = identity(path)?;
    if !cached.exists() {
        return Ok(None);
    }
    anyhow::ensure!(
        cached.metadata()?.len() <= 768 * 1024 * 1024,
        "cache too large"
    );
    let Cached {
        identity: stored,
        animation,
    } = postcard::from_bytes(&fs::read(cached)?)?;
    if stored != identity {
        return Ok(None);
    }
    animation.validate()?;
    Ok(Some(animation))
}
pub fn save(path: &Path, animation: &Animation) -> Result<()> {
    #[derive(Serialize)]
    struct CachedRef<'a> {
        identity: Identity,
        animation: &'a Animation,
    }
    let destination = cache_path(path)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = destination.with_extension(format!("{}.tmp", std::process::id()));
    let file = fs::File::create(&temporary)?;
    use std::io::Write as _;
    let mut writer = postcard::to_io(
        &CachedRef {
            identity: identity(path)?,
            animation,
        },
        std::io::BufWriter::new(file),
    )?;
    writer.flush()?;
    fs::rename(temporary, destination)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_rejects_changed_source_and_corrupt_data() -> Result<()> {
        let directory =
            std::env::temp_dir().join(format!("ghost-cache-test-{}", std::process::id()));
        fs::create_dir_all(&directory)?;
        let source = directory.join("source");
        let cached = directory.join("cache");
        fs::write(&source, b"first")?;
        let animation = Animation {
            width: 1,
            height: 1,
            initial: crate::Frame {
                pixels: vec![255; 4].into_boxed_slice(),
                delay: std::time::Duration::ZERO,
            },
            deltas: vec![],
        };
        fs::write(
            &cached,
            postcard::to_allocvec(&Cached {
                identity: identity(&source)?,
                animation,
            })?,
        )?;
        assert!(read(&source, &cached)?.is_some());
        fs::write(&source, b"replacement")?;
        assert!(read(&source, &cached)?.is_none());
        fs::write(&cached, b"not a cache")?;
        assert!(read(&source, &cached).is_err());
        fs::remove_dir_all(directory)?;
        Ok(())
    }
}
