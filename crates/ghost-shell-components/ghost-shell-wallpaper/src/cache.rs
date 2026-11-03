use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use serde::{Deserialize, Serialize};
use snafu::ResultExt as _;

use crate::Animation;

type Result<T> = std::result::Result<T, Error>;

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
    let path = path.canonicalize().context(IoSnafu {
        operation: "resolve source path",
        path,
    })?;
    let metadata = path.metadata().context(IoSnafu {
        operation: "read source metadata",
        path: &path,
    })?;
    Ok(Identity {
        version: VERSION,
        path: path.clone(),
        length: metadata.len(),
        modified_nanos: metadata
            .modified()
            .context(IoSnafu {
                operation: "read source modification time",
                path: &path,
            })?
            .duration_since(UNIX_EPOCH)
            .context(ModificationTimeSnafu)?
            .as_nanos(),
    })
}
fn cache_path(path: &Path) -> Result<PathBuf> {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    path.canonicalize()
        .context(IoSnafu {
            operation: "resolve source path",
            path,
        })?
        .hash(&mut hash);
    Ok(ghost_shell_config::cache_dir()
        .context(DirectorySnafu)?
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
    snafu::ensure!(
        cached
            .metadata()
            .context(IoSnafu {
                operation: "read cache metadata",
                path: cached
            })?
            .len()
            <= 768 * 1024 * 1024,
        TooLargeSnafu
    );
    let Cached {
        identity: stored,
        animation,
    } = postcard::from_bytes(&fs::read(cached).context(IoSnafu {
        operation: "read cache",
        path: cached,
    })?)
    .context(DecodeSnafu)?;
    if stored != identity {
        return Ok(None);
    }
    animation
        .validate()
        .context(InvalidAnimationSnafu)?;
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
        fs::create_dir_all(parent).context(IoSnafu {
            operation: "create cache directory",
            path: parent,
        })?;
    }
    let temporary = destination.with_extension(format!("{}.tmp", std::process::id()));
    let file = fs::File::create(&temporary).context(IoSnafu {
        operation: "create temporary cache",
        path: &temporary,
    })?;
    use std::io::Write as _;
    let mut writer = postcard::to_io(
        &CachedRef {
            identity: identity(path)?,
            animation,
        },
        std::io::BufWriter::new(file),
    )
    .context(EncodeSnafu)?;
    writer.flush().context(IoSnafu {
        operation: "flush cache",
        path: &temporary,
    })?;
    fs::rename(&temporary, &destination).context(IoSnafu {
        operation: "publish cache",
        path: &destination,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_rejects_changed_source_and_corrupt_data()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
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

#[derive(Debug, snafu::Snafu)]
pub(crate) enum Error {
    #[snafu(display("Could not {operation} at {path:?}"))]
    Io {
        operation: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Wallpaper modification time precedes the Unix epoch"))]
    ModificationTime { source: std::time::SystemTimeError },
    #[snafu(display("Could not resolve wallpaper cache directory"))]
    Directory {
        source: ghost_shell_config::ConfigError,
    },
    #[snafu(display("Wallpaper cache exceeds size limit"))]
    TooLarge,
    #[snafu(display("Could not decode wallpaper cache"))]
    Decode { source: postcard::Error },
    #[snafu(display("Could not encode wallpaper cache"))]
    Encode { source: postcard::Error },
    #[snafu(display("Wallpaper cache contains an invalid animation"))]
    InvalidAnimation { source: crate::Error },
}
