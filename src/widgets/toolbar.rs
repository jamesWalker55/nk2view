// use std::sync::LazyLock;

use iced::alignment::Vertical;
use iced::widget::text::IntoFragment;
use iced::widget::{button, column, container, responsive, row, text};
use iced::{Alignment, Border, Color, Element, Length, Padding, Pixels, Shadow, Task, alignment};

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

fn icon_button<'a>(
    handle: impl Into<iced::widget::image::Handle>,
    msg: Message,
) -> iced::widget::Button<'a, Message> {
    minimal_button(
        container(iced::widget::image(handle))
            .center_x(24.0)
            .center_y(24.0),
    )
    .on_press(msg)
}

fn icon_button_menu<'a>(
    handle: impl Into<iced::widget::image::Handle>,
    msg: Message,
) -> iced::widget::Button<'a, Message> {
    minimal_button(
        row![
            container(iced::widget::image(handle))
                .center_x(16.0)
                .center_y(16.0),
            container(iced::widget::image(&*ICON_CARET_DOWN))
                .center_x(8.0)
                .center_y(6.0),
        ]
        .spacing(2.0)
        .align_y(Vertical::Center),
    )
    .padding(4.0)
    .width(Length::Shrink)
    .on_press(msg)
}

fn icon_text_button<'a>(
    handle: impl Into<iced::widget::image::Handle>,
    content: impl IntoFragment<'a>,
    msg: Message,
) -> iced::widget::Button<'a, Message> {
    minimal_button(
        container(
            row![
                container(iced::widget::image(handle))
                    .center_x(24.0)
                    .center_y(24.0),
                text(content).size(12.0),
            ]
            .align_y(Vertical::Center),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill),
    )
    .width(Length::Shrink)
    .padding(Padding {
        top: 0.0,
        right: 6.0,
        bottom: 0.0,
        left: 0.0, // no left padding because icon is already padded
    })
    .on_press(msg)
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
icon!(ICON_ZOOM_IN, "fluent--zoom-in-16-regular.png");
icon!(ICON_ZOOM_OUT, "fluent--zoom-out-16-regular.png");
icon!(ICON_CARET_DOWN, "fluent--caret-down-16-regular_CROP.png");

pub fn toolbar<'a>() -> impl Into<Element<'a, Message>> {
    responsive(|size| {
        let mut items = vec![
            icon_button(&*ICON_RECONNECT, Message::ReconnectRequested).into(),
            icon_button_menu(&*ICON_CHANNEL, Message::ReconnectRequested).into(),
            icon_text_button(&*ICON_RECONNECT, "Reconnect", Message::ReconnectRequested).into(),
            icon_button(&*ICON_CHANNEL, Message::ReconnectRequested).into(),
            icon_button(&*ICON_ZOOM_IN, Message::ZoomIn).into(),
            icon_button(&*ICON_ZOOM_OUT, Message::ZoomOut).into(),
            icon_button(&*ICON_SAVE, Message::SaveScene).into(),
        ];
        row(items).width(Length::Fill).height(Length::Fill).into()
    })
    .width(Length::Fill)
    .height(24.0)
}
