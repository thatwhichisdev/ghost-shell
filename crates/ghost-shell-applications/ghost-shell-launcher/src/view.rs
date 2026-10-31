use std::sync::Arc;

use iced::{
    Element, Fill, Task,
    widget::{button, column, container, operation, row, text, text_input},
};

use crate::entries::DesktopEntry;

#[derive(Debug, Clone)]
pub enum Message {
    Query(String),
    Move(bool),
    Select(usize),
    Launch(Vec<String>),
    Scrolled(f32),
    Reveal(f32),
    Loaded(Arc<Vec<DesktopEntry>>),
    Failed(String),
}
#[derive(Default)]
pub struct Launcher {
    query: String,
    entries: Arc<Vec<DesktopEntry>>,
    filtered: Vec<usize>,
    selected: Option<usize>,
    offset: f32,
    pub error: Option<String>,
}
impl Launcher {
    pub fn load(theme: Option<String>) -> Task<Message> {
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    Arc::new(crate::entries::load(theme).items)
                })
                .await
                .map_err(|error| error.to_string())
            },
            |result| match result {
                Ok(entries) => Message::Loaded(entries),
                Err(error) => Message::Failed(error),
            },
        )
    }
    pub fn reset(&mut self) -> Task<Message> {
        self.query.clear();
        self.filter();
        operation::focus("launcher-query")
    }
    fn filter(&mut self) {
        self.filtered = if self.query.trim().is_empty() {
            (0..self.entries.len()).collect()
        } else {
            neo_frizbee::match_list(
                self.query.trim(),
                self.entries.as_slice(),
                &neo_frizbee::Config::default(),
            )
            .into_iter()
            .map(|entry| entry.index as usize)
            .collect()
        };
        self.selected = (!self.filtered.is_empty()).then_some(0);
        self.offset = 0.0;
    }
    fn command(&self) -> Option<Vec<String>> {
        let entry = self
            .selected
            .and_then(|index| self.filtered.get(index))
            .and_then(|index| self.entries.get(*index))?;
        if entry.command.is_empty() {
            return None;
        }
        Some(if entry.terminal {
            ["ghostty".to_owned(), "-e".to_owned()]
                .into_iter()
                .chain(entry.command.iter().cloned())
                .collect()
        } else {
            entry.command.clone()
        })
    }
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(entries) => {
                self.entries = entries;
                self.filter();
            }
            Message::Query(query) => {
                self.query = query;
                self.filter();
                return operation::scroll_to(
                    "launcher-list",
                    iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
                );
            }
            Message::Move(forward) => {
                self.selected = ghost_shell_theme::selection(
                    self.selected,
                    self.filtered.len(),
                    forward,
                );
                if let Some(index) = self.selected {
                    return ghost_shell_theme::reveal_list_row(
                        "launcher-list",
                        index as f32 * 48.0,
                        48.0,
                    )
                    .map(Message::Reveal);
                }
            }
            Message::Select(index) => {
                self.selected = Some(index);
            }
            Message::Scrolled(offset) => self.offset = offset,
            Message::Reveal(offset) => {
                self.offset = offset;
                return operation::scroll_to(
                    "launcher-list",
                    iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: offset },
                );
            }
            Message::Failed(error) => self.error = Some(error),
            Message::Launch(_) => {}
        }
        Task::none()
    }
    pub fn view(&self) -> Element<'_, Message> {
        let list = ghost_shell_theme::virtual_list(
            self.filtered.len(),
            self.offset,
            450.0,
            48.0,
            "launcher-list",
            |viewport| Message::Scrolled(viewport.absolute_offset().y),
            |index| {
                let Some(entry) = self
                    .filtered
                    .get(index)
                    .and_then(|index| self.entries.get(*index))
                else {
                    return text("").into();
                };
                let icon: Element<'_, Message> = match &entry.icon {
                    Some(path)
                        if path
                            .extension()
                            .is_some_and(|extension| extension == "svg") =>
                    {
                        iced::widget::svg(path)
                            .width(32)
                            .height(32)
                            .into()
                    }
                    Some(path) => iced::widget::image(path)
                        .width(32)
                        .height(32)
                        .into(),
                    None => iced::widget::Space::new().width(32).into(),
                };
                button(
                    row![
                        icon,
                        column![
                            text(&entry.name)
                                .wrapping(iced::widget::text::Wrapping::None),
                            text(entry.description.as_deref().unwrap_or(""))
                                .size(12)
                                .wrapping(iced::widget::text::Wrapping::None)
                        ]
                    ]
                    .spacing(8),
                )
                .width(Fill)
                .height(Fill)
                .style(move |theme, status| {
                    ghost_shell_theme::search_item(
                        theme,
                        status,
                        self.selected == Some(index),
                    )
                })
                .on_press(Message::Select(index))
                .into()
            },
        );
        let mut input = text_input("Search applications", &self.query)
            .id("launcher-query")
            .style(ghost_shell_theme::search_input)
            .on_input(Message::Query)
            .padding(8);
        if let Some(command) = self.command() {
            input = input.on_submit(Message::Launch(command));
        }
        let mut content = column![
            input,
            list,
            text(format!(
                "{} matches · {} applications",
                self.filtered.len(),
                self.entries.len()
            ))
            .size(12)
        ]
        .spacing(6);
        if let Some(error) = &self.error {
            content = content.push(
                text(error)
                    .style(iced::widget::text::danger)
                    .size(12),
            );
        }
        container(content)
            .padding(8)
            .width(Fill)
            .height(Fill)
            .into()
    }
}
