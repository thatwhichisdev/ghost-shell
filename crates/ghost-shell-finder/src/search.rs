use std::path::PathBuf;

use anyhow::Result;
use fff_search::{
    FilePicker, FilePickerOptions, FuzzySearchOptions, MixedItemRef,
    MixedSearchConfig, PaginationArgs, QueryParser,
};

pub struct Search {
    picker: FilePicker,
    parser: QueryParser<MixedSearchConfig>,
}

pub struct SearchOptions {
    pub(crate) base_path: String,
    pub(crate) enable_content_indexing: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            base_path: "/".to_owned(),
            enable_content_indexing: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchItemKind {
    File,
    Directory,
}

#[derive(Debug, Clone)]
pub struct SearchItem {
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
            enable_fs_root_scanning: true,
            enable_home_dir_scanning: true,
            ..Default::default()
        };

        let picker = FilePicker::new(options)?;
        let parser = QueryParser::new(MixedSearchConfig);

        Ok(Self { picker, parser })
    }

    pub fn index(&mut self) -> Result<()> {
        self.picker.collect_files().map_err(|e| e.into())
    }

    pub fn search(&self, needle: &str, limit: usize) -> Vec<SearchItem> {
        let needle = needle.trim();

        if needle.is_empty() {
            return Vec::new();
        }

        let query = self.parser.parse(needle);

        let results = self.picker.fuzzy_search_mixed(
            &query,
            None,
            FuzzySearchOptions {
                max_threads: 0,
                pagination: PaginationArgs { offset: 0, limit },
                ..Default::default()
            },
        );

        results
            .items
            .into_iter()
            .zip(results.scores)
            .map(|(item, score)| {
                let (path, kind) = match item {
                    MixedItemRef::File(file) => (
                        file.absolute_path(
                            &self.picker,
                            self.picker.base_path(),
                        ),
                        SearchItemKind::File,
                    ),

                    MixedItemRef::Dir(dir) => (
                        dir.absolute_path(
                            &self.picker,
                            self.picker.base_path(),
                        ),
                        SearchItemKind::Directory,
                    ),
                };

                SearchItem {
                    path,
                    kind,
                    score: score.total,
                }
            })
            .collect()
    }
}
