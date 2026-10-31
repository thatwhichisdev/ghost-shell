use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use futures::Stream;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::de::DeserializeOwned;

pub fn changes<T: DeserializeOwned + Send + 'static>(
    name: &'static str,
) -> impl Stream<Item = Result<T, String>> {
    changes_in(name, crate::config_dir().map_err(|error| error.to_string()))
}

fn changes_in<T: DeserializeOwned + Send + 'static>(
    name: &'static str,
    directory: Result<PathBuf, String>,
) -> impl Stream<Item = Result<T, String>> {
    futures::stream::unfold((None, false), move |(state, stopped)| {
        let directory = directory.clone();
        async move {
            if stopped {
                return None;
            }
            let mut state = match state {
                Some(state) => state,
                None => {
                    match directory.and_then(|directory| Watching::new(directory, name)) {
                        Ok(state) => state,
                        Err(error) => return Some((Err(error), (None, true))),
                    }
                }
            };
            if !state.initial {
                if state.receiver.changed().await.is_err() {
                    return None;
                }
                loop {
                    match tokio::time::timeout(
                        Duration::from_millis(150),
                        state.receiver.changed(),
                    )
                    .await
                    {
                        Ok(Ok(())) => continue,
                        Ok(Err(_)) => return None,
                        Err(_) => break,
                    }
                }
            }
            state.initial = false;
            let error = state.receiver.borrow_and_update().clone();
            let result = if let Some(error) = error {
                Err(error)
            } else if let Err(error) = state.refresh() {
                Err(error)
            } else {
                let directory = state.directory.clone();
                let has_file = ["config.toml".to_owned(), format!("{name}.toml")]
                    .iter()
                    .any(|file| {
                        directory
                            .join(file)
                            .symlink_metadata()
                            .is_ok()
                    });
                let removed = state.had_file && !has_file;
                state.had_file |= has_file;
                tokio::task::spawn_blocking(move || {
                    if removed {
                        return Err(
                            "Configuration file was removed; keeping current settings"
                                .into(),
                        );
                    }
                    crate::config::load_directory::<T>(&directory, name)
                        .map_err(|error| error.to_string())
                })
                .await
                .unwrap_or_else(|error| Err(error.to_string()))
            };
            Some((result, (Some(state), false)))
        }
    })
}

struct Watching {
    watcher: RecommendedWatcher,
    receiver: tokio::sync::watch::Receiver<Option<String>>,
    directory: PathBuf,
    watched: Vec<PathBuf>,
    initial: bool,
    had_file: bool,
}

impl Watching {
    fn new(directory: PathBuf, name: &'static str) -> Result<Self, String> {
        let (sender, receiver) = tokio::sync::watch::channel(None);
        let event_directory = directory.clone();
        let watcher =
            notify::recommended_watcher(
                move |event: notify::Result<Event>| match event {
                    Ok(event) if relevant(&event, &event_directory, name) => {
                        sender.send_replace(None);
                    }
                    Ok(_) => {}
                    Err(error) => {
                        sender.send_replace(Some(error.to_string()));
                    }
                },
            )
            .map_err(|error| error.to_string())?;
        let mut state = Self {
            watcher,
            receiver,
            directory,
            watched: Vec::new(),
            initial: true,
            had_file: false,
        };
        state.refresh()?;
        Ok(state)
    }

    fn refresh(&mut self) -> Result<(), String> {
        // Watch directories, not the resolved file: editors and Home Manager
        // replace config.toml (often a symlink) with a new inode on save.
        let paths: Vec<_> = self
            .directory
            .ancestors()
            .filter(|path| path.is_dir())
            .take(2)
            .map(Path::to_path_buf)
            .collect();
        for path in &paths {
            self.watcher
                .watch(path, RecursiveMode::NonRecursive)
                .map_err(|error| error.to_string())?;
        }
        for path in &self.watched {
            if !paths.contains(path) {
                if let Err(error) = self.watcher.unwatch(path) {
                    if path.exists() {
                        return Err(error.to_string());
                    }
                }
            }
        }
        self.watched = paths;
        Ok(())
    }
}

fn relevant(event: &Event, directory: &Path, name: &str) -> bool {
    if matches!(event.kind, EventKind::Access(_)) {
        return false;
    }
    event.paths.is_empty()
        || event.paths.iter().any(|path| {
            path == &directory.join("config.toml")
                || path == &directory.join(format!("{name}.toml"))
                || directory.starts_with(path)
        })
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn replacement_invalid_file_and_recovery()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir()
            .join(format!("ghost-config-watch-{}", std::process::id()));
        std::fs::create_dir_all(&root)?;
        let file = root.join("config.toml");
        std::fs::write(&file, "[bar]\nheight = 28\n")?;
        let stream = changes_in::<crate::BarAppConfig>("bar", Ok(root.clone()));
        futures::pin_mut!(stream);
        let initial = tokio::time::timeout(Duration::from_secs(3), stream.next())
            .await?
            .ok_or("stream ended")??;
        assert_eq!(initial.bar.height, 28.0);

        let next = root.join("next.toml");
        std::fs::write(&next, "[bar]\nheight = 40\n")?;
        std::fs::rename(&next, &file)?;
        let updated = tokio::time::timeout(Duration::from_secs(3), stream.next())
            .await?
            .ok_or("stream ended")??;
        assert_eq!(updated.bar.height, 40.0);

        std::fs::write(&file, "[bar]\nheight = -1\n")?;
        let invalid = tokio::time::timeout(Duration::from_secs(3), stream.next())
            .await?
            .ok_or("stream ended")?;
        assert!(invalid.is_err());

        let source = root.join("nix-generated.toml");
        std::fs::write(&source, "[bar]\nheight = 36\n")?;
        std::os::unix::fs::symlink(&source, &next)?;
        std::fs::rename(&next, &file)?;
        let recovered = tokio::time::timeout(Duration::from_secs(3), stream.next())
            .await?
            .ok_or("stream ended")??;
        assert_eq!(recovered.bar.height, 36.0);

        std::fs::remove_file(&file)?;
        let removed = tokio::time::timeout(Duration::from_secs(3), stream.next())
            .await?
            .ok_or("stream ended")?;
        assert!(removed.is_err());
        std::fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn watches_atomic_replacement_and_directory_recreation() {
        let directory = Path::new("/home/example/.config/ghost-shell");
        for path in [
            directory.join("config.toml"),
            directory.join("bar.toml"),
            directory.to_path_buf(),
            PathBuf::from("/home/example/.config"),
        ] {
            assert!(relevant(
                &Event::new(EventKind::Any).add_path(path),
                directory,
                "bar"
            ));
        }
        assert!(!relevant(
            &Event::new(EventKind::Any).add_path(directory.join("unrelated")),
            directory,
            "bar"
        ));
    }
}
