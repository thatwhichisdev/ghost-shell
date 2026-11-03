use std::path::PathBuf;

use fff_search::{
    FilePicker, FilePickerOptions, FuzzySearchOptions, MixedItemRef, MixedSearchConfig,
    PaginationArgs, QueryParser, ScanProgress, SharedFilePicker, SharedFrecency,
};
use snafu::{OptionExt as _, ResultExt as _};

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub struct Search {
    picker: SharedFilePicker,

    #[allow(unused)]
    frecency: SharedFrecency,
}

pub struct SearchOptions {
    pub(crate) base_path: String,
    pub(crate) enable_content_indexing: bool,
}

impl SearchOptions {
    pub fn home() -> Result<Self> {
        let directories = directories::UserDirs::new().context(MissingHomeSnafu)?;
        let home = directories
            .home_dir()
            .to_str()
            .context(InvalidHomeEncodingSnafu)?;
        snafu::ensure!(home != "/", RootHomeSnafu);
        Ok(Self {
            base_path: home.to_owned(),
            enable_content_indexing: false,
        })
    }
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct SearchResult {
    pub items: Vec<SearchItem>,
    pub matched: usize,
    pub indexed_files: usize,
    pub indexed_dirs: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchItemKind {
    File,
    Directory,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct SearchItem {
    pub name: String,
    pub path: PathBuf,
    pub kind: SearchItemKind,
    pub score: i32,
}

impl Search {
    pub fn try_new(options: SearchOptions) -> Result<Self> {
        let options = FilePickerOptions {
            base_path: options.base_path,
            enable_mmap_cache: false,
            enable_content_indexing: options.enable_content_indexing,
            watch: true,
            follow_symlinks: false,
            enable_fs_root_scanning: false,
            enable_home_dir_scanning: true,
            ..Default::default()
        };

        let picker = SharedFilePicker::default();
        let frecency = SharedFrecency::default();

        FilePicker::new_with_shared_state(picker.clone(), frecency.clone(), options)
            .context(InitializeSnafu)?;

        Ok(Self { picker, frecency })
    }

    pub fn get_scan_progress(&self) -> Result<ScanProgress> {
        let guard = self
            .picker
            .read()
            .map_err(|_| Error::PickerPoisoned)?;

        let picker = guard
            .as_ref()
            .context(PickerUninitializedSnafu)?;

        Ok(picker.get_scan_progress())
    }

    pub fn search(&self, needle: &str, limit: usize) -> Result<SearchResult> {
        let needle = needle.trim();

        if needle.is_empty() {
            return Ok(SearchResult {
                items: Vec::new(),
                matched: 0,
                indexed_files: 0,
                indexed_dirs: 0,
            });
        }

        let picker_guard = self
            .picker
            .read()
            .map_err(|_| Error::PickerPoisoned)?;

        let picker = picker_guard
            .as_ref()
            .context(PickerUninitializedSnafu)?;

        let query_parser = QueryParser::new(MixedSearchConfig);
        let query = query_parser.parse(needle);

        let results = picker.fuzzy_search_mixed(
            &query,
            None,
            FuzzySearchOptions {
                max_threads: 0,
                pagination: PaginationArgs { offset: 0, limit },
                ..Default::default()
            },
        );

        let items = results
            .items
            .into_iter()
            .zip(results.scores)
            .map(|(item, score)| {
                let (name, path, kind) = match item {
                    MixedItemRef::File(file) => (
                        file.file_name(picker),
                        file.absolute_path(picker, picker.base_path()),
                        SearchItemKind::File,
                    ),
                    MixedItemRef::Dir(dir) => (
                        dir.dir_name(picker),
                        dir.absolute_path(picker, picker.base_path()),
                        SearchItemKind::Directory,
                    ),
                };

                SearchItem {
                    name,
                    path,
                    kind,
                    score: score.total,
                }
            })
            .collect();

        Ok(SearchResult {
            items,
            matched: results.total_matched,
            indexed_files: results.total_files,
            indexed_dirs: results.total_dirs,
        })
    }
}

#[derive(Debug, snafu::Snafu)]
pub enum Error {
    #[snafu(display("Could not resolve the user’s home directory"))]
    MissingHome,
    #[snafu(display("Home directory is not valid UTF-8"))]
    InvalidHomeEncoding,
    #[snafu(display("Refusing to index the filesystem root as the home directory"))]
    RootHome,
    #[snafu(display("File picker lock is poisoned"))]
    PickerPoisoned,
    #[snafu(display("File picker is not initialized"))]
    PickerUninitialized,
    #[snafu(display("Could not initialize file search"))]
    Initialize { source: fff_search::Error },
}
