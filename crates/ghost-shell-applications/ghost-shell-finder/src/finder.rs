mod search;
use std::sync::Arc;

use iced::{
    Element, Fill, Task,
    widget::{button, column, container, operation, scrollable, text, text_input},
};
use search::{Search, SearchOptions, SearchResult};

#[derive(Clone)]
pub enum Message {
    Ready(Result<Arc<Search>, String>),
    Query(String),
    Search,
    Results(u64, Result<SearchResult, String>),
    Progress(Result<(bool, usize), String>),
    Move(bool),
    Select(usize),
    Reveal(f32),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("FinderMessage")
    }
}

#[derive(Default)]
pub struct Finder {
    search: Option<Arc<Search>>,
    query: String,
    generation: u64,
    result: Option<SearchResult>,
    selected: Option<usize>,
    indexing: bool,
    searching: bool,
    pending_search: bool,
    indexed: usize,
    error: Option<String>,
}

impl Finder {
    pub fn start(&mut self) -> Task<Message> {
        if self.search.is_some() || self.indexing {
            return operation::focus("finder-query");
        }
        self.indexing = true;
        Task::perform(
            async {
                tokio::task::spawn_blocking(|| {
                    SearchOptions::home()
                        .and_then(Search::try_new)
                        .map(Arc::new)
                        .map_err(|error| error.to_string())
                })
                .await
                .unwrap_or_else(|error| Err(error.to_string()))
            },
            Message::Ready,
        )
    }

    fn progress(&self) -> Task<Message> {
        let Some(search) = self.search.clone() else {
            return Task::none();
        };
        Task::perform(
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                tokio::task::spawn_blocking(move || {
                    search
                        .get_scan_progress()
                        .map(|progress| {
                            (progress.is_scanning, progress.scanned_files_count)
                        })
                        .map_err(|error| error.to_string())
                })
                .await
                .unwrap_or_else(|error| Err(error.to_string()))
            },
            Message::Progress,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Ready(Ok(search)) => {
                self.search = Some(search);
                return self.progress();
            }
            Message::Ready(Err(error)) => {
                self.error = Some(error);
                self.indexing = false;
            }
            Message::Query(query) => self.query = query,
            Message::Progress(Ok((scanning, count))) => {
                self.indexing = scanning;
                self.indexed = count;
                if scanning {
                    return self.progress();
                }
                if self.pending_search {
                    self.pending_search = false;
                    return self.update(Message::Search);
                }
            }
            Message::Progress(Err(error)) => {
                self.indexing = false;
                self.error = Some(error);
            }
            Message::Search => {
                if self.indexing {
                    self.pending_search = true;
                    return Task::none();
                }
                let Some(search) = self.search.clone() else {
                    return Task::none();
                };
                self.generation = self.generation.wrapping_add(1);
                let generation = self.generation;
                let query = self.query.clone();
                self.searching = true;
                self.error = None;
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            search
                                .search(&query, 100)
                                .map_err(|error| error.to_string())
                        })
                        .await
                        .unwrap_or_else(|error| Err(error.to_string()))
                    },
                    move |result| Message::Results(generation, result),
                );
            }
            Message::Results(generation, result) if generation == self.generation => {
                self.searching = false;
                match result {
                    Ok(result) => {
                        self.selected = (!result.items.is_empty()).then_some(0);
                        self.indexed = result.indexed_files;
                        self.result = Some(result);
                        return operation::scroll_to(
                            "finder-list",
                            iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
                        );
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Message::Results(..) => {}
            Message::Move(forward) => {
                let count = self
                    .result
                    .as_ref()
                    .map_or(0, |result| result.items.len());
                self.selected =
                    ghost_shell_theme::selection(self.selected, count, forward);
                if self.selected.is_some() {
                    return ghost_shell_theme::reveal_list_item(
                        "finder-list",
                        "finder-selected",
                    )
                    .map(Message::Reveal);
                }
            }
            Message::Select(index) => self.selected = Some(index),
            Message::Reveal(offset) => {
                return operation::scroll_to(
                    "finder-list",
                    iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: offset },
                );
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let count = self
            .result
            .as_ref()
            .map_or(0, |result| result.items.len());

        let list = scrollable(column((0..count).map(|index| {
            let label = self
                .result
                .as_ref()
                .and_then(|result| result.items.get(index))
                .map(|item| item.path.to_string_lossy().into_owned())
                .unwrap_or_default();
            let item = button(
                text(label)
                    .width(Fill)
                    .wrapping(iced::widget::text::Wrapping::Glyph)
                    .line_height(iced::widget::text::LineHeight::Absolute(24.0.into())),
            )
            .padding([0, 5])
            .width(Fill)
            .style(move |theme, status| {
                ghost_shell_theme::search_item(
                    theme,
                    status,
                    self.selected == Some(index),
                )
            })
            .on_press(Message::Select(index));
            let item = container(item).width(Fill);
            if self.selected == Some(index) {
                item.id("finder-selected").into()
            } else {
                item.into()
            }
        })))
        .id("finder-list")
        .style(ghost_shell_theme::search_scrollable)
        .height(Fill);

        let status = if self.indexing {
            format!("Indexing… {} files", self.indexed)
        } else if self.searching {
            "Searching…".to_owned()
        } else {
            format!(
                "{} matches · {} files",
                self.result
                    .as_ref()
                    .map_or(0, |result| result.matched),
                self.indexed
            )
        };

        let input = text_input("Search files (Enter)", &self.query)
            .id("finder-query")
            .style(ghost_shell_theme::search_input)
            .padding(8);

        let input = input
            .on_input(Message::Query)
            .on_submit(Message::Search);

        let mut content = column![input, list, text(status).size(12)].spacing(6);
        if let Some(error) = &self.error {
            content = content.push(
                text(error)
                    .style(iced::widget::text::danger)
                    .size(12),
            );
        }

        container(content)
            .padding(8)
            .height(Fill)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_search_completion_cannot_replace_current_results() {
        let mut finder = Finder {
            generation: 2,
            searching: true,
            ..Default::default()
        };
        drop(finder.update(Message::Results(1, Err("old failure".into()))));
        assert!(finder.searching);
        assert!(finder.error.is_none());
        drop(finder.update(Message::Results(
            2,
            Ok(SearchResult {
                items: Vec::new(),
                matched: 0,
                indexed_files: 42,
                indexed_dirs: 3,
            }),
        )));
        assert!(!finder.searching);
        assert_eq!(finder.indexed, 42);
        assert!(finder.selected.is_none());
    }
}
