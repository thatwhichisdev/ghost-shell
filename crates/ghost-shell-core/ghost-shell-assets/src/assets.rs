#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "**/*.svg"]
pub struct Assets;
pub fn icon<'a, Message: 'a>(name: &str, size: f32) -> iced::Element<'a, Message> {
    match Assets::get(name) {
        Some(asset) => iced::widget::svg(iced::widget::svg::Handle::from_memory(
            asset.data.into_owned(),
        ))
        .width(size)
        .height(size)
        .style(|theme: &iced::Theme, _| iced::widget::svg::Style {
            color: Some(theme.palette().text),
        })
        .into(),
        None => iced::widget::text("•").into(),
    }
}
