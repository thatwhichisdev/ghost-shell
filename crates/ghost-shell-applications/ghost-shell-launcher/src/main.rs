use snafu::{OptionExt as _, ResultExt as _};
mod app;

#[snafu::report]
fn main() -> Result<(), Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    let runtime =
        std::env::var_os("XDG_RUNTIME_DIR").context(MissingRuntimeDirectorySnafu)?;
    let path = std::path::PathBuf::from(runtime).join("ghost-shell-launcher.lock");
    let instance = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .context(OpenInstanceLockSnafu { path: &path })?;
    match instance.try_lock() {
        Ok(()) => app::run(),
        Err(std::fs::TryLockError::WouldBlock) => Ok(()),
        Err(std::fs::TryLockError::Error(error)) => Err(Error::LockInstance {
            path,
            source: error,
        }),
    }
}

#[derive(Debug, snafu::Snafu)]
pub enum Error {
    #[snafu(display("Could not run Ghost launcher"))]
    Runtime { source: iced_exwlshell::Error },
    #[snafu(display("XDG_RUNTIME_DIR is not set"))]
    MissingRuntimeDirectory,
    #[snafu(display("Could not open instance lock {path:?}"))]
    OpenInstanceLock {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Could not acquire instance lock {path:?}"))]
    LockInstance {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}
