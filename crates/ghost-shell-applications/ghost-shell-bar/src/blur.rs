use ghost_shell_theme::bar::Decoration;
use iced_exwlshell::reexport::BlurRegion;

pub fn regions(decorations: &[Decoration]) -> Vec<BlurRegion> {
    let mut regions: Vec<BlurRegion> = Vec::new();
    for decoration in decorations {
        let bounds = decoration.bounds;
        let limit = (bounds.width.min(bounds.height) / 2.0).max(0.0);
        let radius = decoration.radius;
        let corners = [
            radius.top_left,
            radius.top_right,
            radius.bottom_left,
            radius.bottom_right,
        ]
        .map(|radius| radius.clamp(0.0, limit));
        // Wayland blur regions are integer rectangles. Keep each scanline inside
        // the rounded background so blur cannot leak into the transparent gaps.
        for y in bounds.y.ceil() as i32..(bounds.y + bounds.height).floor() as i32 {
            let top = y as f32 - bounds.y;
            let bottom = bounds.y + bounds.height - (y + 1) as f32;
            let left = inset(corners[0], top).max(inset(corners[2], bottom));
            let right = inset(corners[1], top).max(inset(corners[3], bottom));
            let x = (bounds.x + left).ceil() as i32;
            let end = (bounds.x + bounds.width - right).floor() as i32;
            if end <= x {
                continue;
            }
            if let Some(previous) = regions.last_mut()
                && previous.x == x
                && previous.width == end - x
                && previous.y + previous.height == y
            {
                previous.height += 1;
            } else {
                regions.push(BlurRegion {
                    x,
                    y,
                    width: end - x,
                    height: 1,
                });
            }
        }
    }
    regions
}

fn inset(radius: f32, distance: f32) -> f32 {
    if distance >= radius {
        return 0.0;
    }
    radius
        - (radius * radius - (radius - distance).powi(2))
            .max(0.0)
            .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_preserve_gaps_and_rounded_corners() {
        let decorations = [0.0, 30.0].map(|x| Decoration {
            bounds: iced::Rectangle::new(
                iced::Point::new(x, 4.0),
                iced::Size::new(20.0, 20.0),
            ),
            radius: 4.0.into(),
        });
        let regions = regions(&decorations);
        assert!(
            regions
                .iter()
                .all(|region| region.x + region.width <= 20 || region.x >= 30)
        );
        assert!(
            regions
                .iter()
                .filter(|region| region.y == 4)
                .all(|region| region.width <= 12)
        );
        assert!(
            regions
                .iter()
                .any(|region| region.width == 20)
        );
    }

    #[test]
    fn square_decoration_coalesces_to_one_region() {
        let result = regions(&[Decoration {
            bounds: iced::Rectangle::new(
                iced::Point::new(9.0, 4.0),
                iced::Size::new(20.0, 20.0),
            ),
            radius: 0.0.into(),
        }]);
        assert_eq!(
            result,
            vec![BlurRegion {
                x: 9,
                y: 4,
                width: 20,
                height: 20
            }]
        );
        assert!(regions(&[]).is_empty());
    }
}
