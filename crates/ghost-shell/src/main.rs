use anyhow::Result;
use gpui::{App, AssetSource, SharedString};
use std::{fs, path::PathBuf};

struct Assets {
    base: PathBuf,
}

impl AssetSource for Assets {
    fn load(
        &self,
        path: &str,
    ) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        fs::read(self.base.join(path))
            .map(|data| Some(std::borrow::Cow::Owned(data)))
            .map_err(|err| err.into())
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        fs::read_dir(self.base.join(path))
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        entry
                            .ok()
                            .and_then(|entry| {
                                entry.file_name().into_string().ok()
                            })
                            .map(SharedString::from)
                    })
                    .collect()
            })
            .map_err(|err| err.into())
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(Assets {
        base: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets"),
    });

    app.run(|cx: &mut App| {
        if let Err(err) = ghost_shell_app::init(cx) {
            eprintln!("App initialization failed {err:#}");
            cx.quit();
        } else {
            cx.activate(true);
        }
    });
}
