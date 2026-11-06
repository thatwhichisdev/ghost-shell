use std::{collections::VecDeque, time::Duration};

use ghost_shell_system::{SystemInfo, Update};
use ghost_shell_theme::bar::{self, Style};
use iced::{
    Element, Point, Rectangle, Renderer, Theme, mouse,
    widget::{
        Space,
        canvas::{self, Canvas, Frame, Geometry, Path, Stroke},
        row,
    },
};

const WINDOW: Duration = Duration::from_secs(60);
const MAX_SAMPLES: usize = 61;

#[derive(Default)]
pub struct SystemMonitor {
    history: VecDeque<SystemInfo>,
}

impl SystemMonitor {
    pub fn update(&mut self, update: Update) {
        match update {
            Ok(sample) => {
                while self.history.front().is_some_and(|old| {
                    sample
                        .sampled_at
                        .saturating_duration_since(old.sampled_at)
                        > WINDOW
                }) || self.history.len() >= MAX_SAMPLES
                {
                    self.history.pop_front();
                }
                self.history.push_back(sample);
            }
            Err(error) => {
                self.history.clear();
                log::warn!(
                    "System monitoring: {}",
                    snafu::Report::from_error(error.as_ref())
                );
            }
        }
    }

    pub fn view<Message: 'static>(&self, style: Style) -> Element<'_, Message> {
        row([
            self.section("icons/cpu-fill.svg", Metric::Cpu, style),
            self.section("icons/graphics-card-fill.svg", Metric::Gpu, style),
            self.section("icons/memory-fill.svg", Metric::Memory, style),
        ])
        .spacing(0)
        .align_y(iced::Center)
        .into()
    }

    fn section<Message: 'static>(
        &self,
        icon: &str,
        metric: Metric,
        style: Style,
    ) -> Element<'_, Message> {
        if self
            .history
            .back()
            .and_then(|sample| metric.value(sample))
            .is_none()
        {
            return Space::new().width(0).height(0).into();
        }
        let graph = Canvas::new(Graph {
            history: &self.history,
            metric,
        })
        .width(30.0)
        .height((style.widgets.height - 4.0).max(1.0));
        bar::outer(
            bar::inner(
                row![bar::icon(icon, style.widgets.icon_size), graph]
                    .spacing(style.widgets.padding_horizontal)
                    .align_y(iced::Center),
                style,
            ),
            style,
        )
    }
}

#[derive(Clone, Copy)]
enum Metric {
    Cpu,
    Gpu,
    Memory,
}

impl Metric {
    fn value(self, sample: &SystemInfo) -> Option<f32> {
        match self {
            Self::Cpu => sample.cpu_usage,
            Self::Gpu => sample.gpu_usage,
            Self::Memory => sample.memory_usage(),
        }
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 100.0))
    }
}

struct Graph<'a> {
    history: &'a VecDeque<SystemInfo>,
    metric: Metric,
}

impl<Message> canvas::Program<Message> for Graph<'_> {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let foreground = theme
            .extended_palette()
            .background
            .neutral
            .color;
        let width = (bounds.width - 1.0).max(0.0);
        let height = (bounds.height - 1.0).max(0.0);
        let Some(latest) = self.history.back() else {
            return vec![frame.into_geometry()];
        };
        let mut segment = Vec::with_capacity(MAX_SAMPLES);
        let paint = |frame: &mut Frame, points: &mut Vec<Point>| {
            if let (Some(first), Some(last)) = (points.first(), points.last()) {
                let line = Path::new(|builder| {
                    builder.move_to(*first);
                    for point in points.iter().skip(1) {
                        builder.line_to(*point);
                    }
                });
                let area = Path::new(|builder| {
                    builder.move_to(Point::new(first.x, height + 0.5));
                    for point in points.iter() {
                        builder.line_to(*point);
                    }
                    builder.line_to(Point::new(last.x, height + 0.5));
                    builder.close();
                });
                frame.fill(&area, foreground.scale_alpha(0.25));
                frame.stroke(
                    &line,
                    Stroke::default()
                        .with_color(foreground)
                        .with_width(1.0),
                );
            }
            points.clear();
        };
        let mut previous = None;
        for sample in self.history {
            if previous.is_some_and(|time| {
                sample
                    .sampled_at
                    .saturating_duration_since(time)
                    > Duration::from_secs(2)
            }) {
                paint(&mut frame, &mut segment);
            }
            previous = Some(sample.sampled_at);
            match self.metric.value(sample) {
                Some(value) => {
                    let age = latest
                        .sampled_at
                        .saturating_duration_since(sample.sampled_at)
                        .as_secs_f32();
                    segment.push(Point::new(
                        0.5 + width * (1.0 - age / WINDOW.as_secs_f32()).clamp(0.0, 1.0),
                        0.5 + height * (1.0 - value / 100.0),
                    ));
                }
                None => paint(&mut frame, &mut segment),
            }
        }
        paint(&mut frame, &mut segment);
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    fn sample(sampled_at: Instant) -> SystemInfo {
        SystemInfo {
            sampled_at,
            cpu_usage: Some(50.0),
            gpu_usage: None,
            mem_usage: 25,
            mem_total: 100,
            gpu_error: None,
        }
    }

    #[test]
    fn history_is_bounded_and_expires_after_suspend() {
        let start = Instant::now();
        let mut monitor = SystemMonitor::default();
        for second in 0..120 {
            monitor.update(Ok(sample(start + Duration::from_secs(second))));
        }
        assert_eq!(monitor.history.len(), 61);
        monitor.update(Ok(sample(start + Duration::from_secs(300))));
        assert_eq!(monitor.history.len(), 1);
    }

    #[test]
    fn unavailable_and_invalid_values_are_not_zero() {
        let mut sample = sample(Instant::now());
        assert_eq!(Metric::Gpu.value(&sample), None);
        assert_eq!(Metric::Memory.value(&sample), Some(25.0));
        sample.mem_total = 0;
        sample.cpu_usage = Some(f32::NAN);
        assert_eq!(Metric::Memory.value(&sample), None);
        assert_eq!(Metric::Cpu.value(&sample), None);
        sample.gpu_usage = Some(150.0);
        assert_eq!(Metric::Gpu.value(&sample), Some(100.0));
        sample.gpu_usage = Some(0.0);
        assert_eq!(Metric::Gpu.value(&sample), Some(0.0));
    }
}
