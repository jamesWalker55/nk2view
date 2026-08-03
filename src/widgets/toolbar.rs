// use std::sync::LazyLock;

use iced::widget::{button, column, container, responsive, row, text};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Task, alignment};

use crate::Message;

fn minimal_button<'a>(
    content: impl Into<Element<'a, Message, iced::Theme, iced::Renderer>>,
) -> iced::widget::Button<'a, Message, iced::Theme, iced::Renderer> {
    use iced::widget::button::Style;
    button(content)
        .padding(0.0)
        .height(24.0)
        .width(24.0)
        .clip(true)
        .style(|_theme, status| {
            let border = Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            };
            let shadow = Shadow::default();
            match status {
                button::Status::Active => Style {
                    background: None,
                    text_color: Color::BLACK,
                    border,
                    shadow,
                    snap: true,
                },
                button::Status::Hovered => Style {
                    background: Some(iced::Background::Color(Color::BLACK.scale_alpha(0.06))),
                    text_color: Color::BLACK,
                    border,
                    shadow,
                    snap: true,
                },
                button::Status::Pressed => Style {
                    background: Some(iced::Background::Color(Color::BLACK.scale_alpha(0.3))),
                    text_color: Color::BLACK,
                    border,
                    shadow,
                    snap: true,
                },
                button::Status::Disabled => Style {
                    background: Some(iced::Background::Color(Color::BLACK.scale_alpha(0.1))),
                    text_color: Color::BLACK.scale_alpha(0.7),
                    border,
                    shadow,
                    snap: true,
                },
            }
        })
}

macro_rules! icon {
    ($name:ident, $filename:literal) => {
        static $name: std::sync::LazyLock<iced::widget::image::Handle> =
            std::sync::LazyLock::new(|| {
                iced::widget::image::Handle::from_bytes(
                    include_bytes!(concat!("../../icons/", $filename)).as_slice(),
                )
            });
    };
}

icon!(ICON_RECONNECT, "fluent--arrow-clockwise-16-regular.png");
icon!(ICON_CHANNEL, "fluent--midi-16-regular.png");
icon!(ICON_SAVE, "fluent--save-16-regular.png");

pub fn toolbar<'a>() -> impl Into<Element<'a, Message>> {
    responsive(|size| {
        let mut items = vec![
            minimal_button(
                container(iced::widget::image(&*ICON_RECONNECT))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
            .on_press(Message::ReconnectRequested)
            .into(),
            minimal_button(
                container(iced::widget::image(&*ICON_CHANNEL))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
            .on_press(Message::ReconnectRequested)
            .into(),
            minimal_button(
                container(iced::widget::image(&*ICON_SAVE))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
            .on_press(Message::ReconnectRequested)
            .into(),
        ];
        row(items).width(Length::Fill).height(Length::Fill).into()
    })
    .width(Length::Fill)
    .height(24.0)
}
