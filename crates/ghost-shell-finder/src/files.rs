use anyhow::{Context as _, Result};
use fff_search::{
    FilePicker, FilePickerOptions, SharedFilePicker, SharedFrecency,
};
use gpui::Global;

#[derive(Clone, Default)]
pub struct FileSearch {
    picker: SharedFilePicker,
    frecency: SharedFrecency,
}

impl FileSearch {
    pub fn start(&self) -> Result<()> {
        FilePicker::new_with_shared_state(
            self.picker.clone(),
            self.frecency.clone(),
            FilePickerOptions {
                base_path: "/".to_owned(),

                // We only need filename/path searching for the launcher.
                enable_mmap_cache: false,
                enable_content_indexing: false,

                // Keep the index synchronized after the initial scan.
                watch: true,

                // Avoid traversing the same trees through symlink aliases.
                follow_symlinks: false,

                // Explicitly required by FFF when base_path == "/".
                enable_fs_root_scanning: true,

                ..Default::default()
            },
        )
        .context("failed to start filesystem index")
    }

    #[must_use]
    pub fn picker(&self) -> &SharedFilePicker {
        &self.picker
    }

    #[must_use]
    pub fn frecency(&self) -> &SharedFrecency {
        &self.frecency
    }
}

impl Global for FileSearch {}
