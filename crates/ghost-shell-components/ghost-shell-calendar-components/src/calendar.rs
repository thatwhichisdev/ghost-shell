use iced::{
    Element, Fill, Size,
    mouse::ScrollDelta,
    widget::{button, column, container, mouse_area, row, sensor},
};
use jiff::{ToSpan, civil::Date};

#[derive(Clone, Debug)]
pub enum Message {
    Navigate(i32),
    Scroll(ScrollDelta),
    Resize(Size),
}

pub struct Calendar {
    month: Date,
    scroll: f32,
}

impl Calendar {
    pub fn new(today: Date) -> Self {
        Self {
            month: today.first_of_month(),
            scroll: 0.0,
        }
    }

    pub fn update(&mut self, message: Message) {
        let months = match message {
            Message::Navigate(months) => {
                self.scroll = 0.0;
                months
            }
            Message::Scroll(delta) => {
                let amount = match delta {
                    ScrollDelta::Lines { y, .. } => y,
                    ScrollDelta::Pixels { y, .. } => y / 60.0,
                };
                if !amount.is_finite() {
                    return;
                }
                self.scroll += amount;
                let steps = self.scroll.trunc();
                self.scroll -= steps;
                (-steps) as i32
            }
            Message::Resize(_) => return,
        };
        if let Some(month) = self.shifted(months) {
            self.month = month;
        }
    }

    fn shifted(&self, months: i32) -> Option<Date> {
        // Reaching Jiff's supported date boundary disables further navigation.
        let span = jiff::Span::new().try_months(months).ok()?;
        self.month.checked_add(span).ok()
    }

    fn days(&self) -> impl Iterator<Item = Option<Date>> + '_ {
        let offset = i64::from(self.month.weekday().to_monday_zero_offset());
        (0..42).map(move |index| {
            self.month
                .checked_add((index - offset).days())
                .ok()
        })
    }

    pub fn size(font_size: f32) -> Size {
        Size::new(
            7.0 * (font_size * 3.5).ceil().max(44.0) + 16.0,
            8.0 * (font_size * 1.6).ceil().max(28.0) + 16.0,
        )
    }

    pub fn view(
        &self,
        today: Date,
        font: iced::Font,
        font_size: f32,
        opacity: f32,
    ) -> Element<'_, Message> {
        let size = Self::size(font_size);
        let cell_width = (size.width - 16.0) / 7.0;
        let cell_height = (size.height - 16.0) / 8.0;
        let label = move |value: String| ghost_shell_theme::label(value, font, font_size);
        let navigation = |amount, symbol: &str| {
            button(
                container(label(symbol.to_owned()))
                    .center_x(Fill)
                    .center_y(Fill),
            )
            .width(cell_height)
            .height(cell_height)
            .padding(0)
            .style(ghost_shell_theme::menu_item)
            .on_press_maybe(
                self.shifted(amount)
                    .map(|_| Message::Navigate(amount)),
            )
        };
        let month = row![
            navigation(-1, "‹"),
            container(label(self.month.strftime("%B").to_string()))
                .center_x(Fill)
                .center_y(cell_height),
            navigation(1, "›")
        ]
        .width(Fill);
        let year = row![
            navigation(-12, "‹"),
            container(label(self.month.year().to_string()))
                .center_x(Fill)
                .center_y(cell_height),
            navigation(12, "›")
        ]
        .width(Fill);
        let mut content = column![row![month, year].height(cell_height)];
        content =
            content.push(row(["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].map(|day| {
                container(label(day.into()))
                    .center_x(cell_width)
                    .center_y(cell_height)
                    .into()
            })));
        let mut days = self.days();
        for _ in 0..6 {
            let mut week = row![];
            for _ in 0..7 {
                let date = days.next().flatten();
                let current_month =
                    date.is_some_and(|date| date.month() == self.month.month());
                let is_today = date == Some(today);
                week = week.push(
                    container(label(
                        date.map(|date| date.day().to_string())
                            .unwrap_or_default(),
                    ))
                    .center_x(cell_width)
                    .center_y(cell_height)
                    .style(move |theme: &iced::Theme| {
                        let mut style = container::Style::default();
                        if is_today {
                            style.background = Some(
                                theme
                                    .extended_palette()
                                    .background
                                    .neutral
                                    .color
                                    .into(),
                            );
                            style.text_color = Some(
                                theme
                                    .extended_palette()
                                    .background
                                    .neutral
                                    .text,
                            );
                            style.border.radius = 4.0.into();
                        } else if !current_month {
                            style.text_color =
                                Some(theme.extended_palette().secondary.base.color);
                        }
                        style
                    }),
                );
            }
            content = content.push(week);
        }
        sensor(
            mouse_area(
                container(content)
                    .padding(8)
                    .width(size.width)
                    .height(size.height)
                    .style(move |theme| {
                        ghost_shell_theme::menu_surface_with_opacity(theme, opacity)
                    }),
            )
            .on_scroll(Message::Scroll),
        )
        .on_show(Message::Resize)
        .on_resize(Message::Resize)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leap_month_grid_and_year_navigation() {
        let mut calendar = Calendar::new(Date::new(2024, 2, 29).unwrap());
        let dates: Vec<_> = calendar.days().flatten().collect();
        assert_eq!(dates.len(), 42);
        assert_eq!(dates[0], Date::new(2024, 1, 29).unwrap());
        assert_eq!(
            dates
                .iter()
                .filter(|date| date.month() == 2)
                .count(),
            29
        );
        calendar.update(Message::Navigate(12));
        assert_eq!(calendar.month, Date::new(2025, 2, 1).unwrap());
        calendar.update(Message::Navigate(-2));
        assert_eq!(calendar.month, Date::new(2024, 12, 1).unwrap());
    }

    #[test]
    fn pixel_scroll_accumulates_and_boundaries_are_safe() {
        let mut calendar = Calendar::new(Date::new(2024, 1, 31).unwrap());
        for _ in 0..2 {
            calendar.update(Message::Scroll(ScrollDelta::Pixels { x: 0.0, y: -20.0 }));
        }
        assert_eq!(calendar.month.month(), 1);
        calendar.update(Message::Scroll(ScrollDelta::Pixels { x: 0.0, y: -20.0 }));
        assert_eq!(calendar.month.month(), 2);
        let mut calendar = Calendar::new(Date::MAX);
        calendar.update(Message::Navigate(1));
        assert_eq!(calendar.month, Date::MAX.first_of_month());
        assert!(calendar.days().any(|date| date.is_none()));
    }
}
