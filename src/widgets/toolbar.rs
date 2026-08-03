// use std::sync::LazyLock;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::text::IntoFragment;
use iced::widget::{Space, button, column, container, responsive, row, space, text};
use iced::{Alignment, Border, Color, Element, Length, Padding, Pixels, Shadow, Task, alignment};

use crate::{ConnectedState, Menu, Message};

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
) -> iced::widget::Button<'a, Message> {
    minimal_button(
        container(iced::widget::image(handle))
            .center_x(24.0)
            .center_y(24.0),
    )
}

fn icon_button_menu<'a>(
    handle: impl Into<iced::widget::image::Handle>,
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
}

fn icon_text_button<'a>(
    handle: impl Into<iced::widget::image::Handle>,
    content: impl IntoFragment<'a>,
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

fn build_channel_menu_button<'a>(current_ch: u8) -> impl Into<Element<'a, Message>> {
    minimal_button(
        row![
            container(iced::widget::image(&*ICON_CHANNEL))
                .center_x(16.0)
                .center_y(16.0),
            container(text(format!("{}", current_ch + 1)).size(12.0))
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
    .on_press(Message::ToggleMenu(Menu::Channel))
}

pub fn toolbar<'a>(active_ch: u8) -> impl Into<Element<'a, Message>> {
    responsive(move |size| {
        let mut items = vec![
            icon_button(&*ICON_RECONNECT)
                .on_press(Message::ReconnectRequested)
                .into(),
            build_channel_menu_button(active_ch).into(),
            icon_text_button(&*ICON_RECONNECT, "Reconnect")
                .on_press(Message::ReconnectRequested)
                .into(),
            icon_button(&*ICON_CHANNEL)
                // disabled button
                .into(),
            icon_button(&*ICON_ZOOM_IN).on_press(Message::ZoomIn).into(),
            icon_button(&*ICON_ZOOM_OUT)
                .on_press(Message::ZoomOut)
                .into(),
            icon_button(&*ICON_SAVE).on_press(Message::SaveScene).into(),
        ];
        row(items).width(Length::Fill).height(Length::Fill).into()
    })
    .width(Length::Fill)
    .height(24.0)
}

pub fn build_menu_ui<'a>(menu: Menu, active_ch: u8) -> impl Into<Element<'a, Message>> {
    match menu {
        Menu::Channel => {
            fn channel_button<'a>(label: String, ch: u8) -> iced::widget::Button<'a, Message> {
                minimal_button(
                    container(text(label).size(12.0))
                        .center_x(Length::Fill)
                        .center_y(Length::Fill),
                )
                .on_press(Message::SetChannel(ch))
            }

            let channel_buttons = column([8..16u8, 0..8u8].map(|range| {
                row(range.map(|ch| {
                    let btn = channel_button(format!("{}", ch + 1), ch);
                    if ch == active_ch {
                        container(btn)
                            .style(|_| {
                                container::Style::default()
                                    .background(Color::from_rgb(0.27, 0.96, 0.35))
                            })
                            .into()
                    } else {
                        btn.into()
                    }
                }))
                .into()
            }));
            let menu_button = build_channel_menu_button(active_ch).into();

            container(
                column![
                    Space::new().height(Length::Fill),
                    container(channel_buttons)
                        .style(|_| { container::Style::default().background(Color::WHITE) }),
                    row![
                        Space::new().width(24.0),
                        container(menu_button)
                            .style(|_| { container::Style::default().background(Color::WHITE) }),
                    ]
                    .align_y(Vertical::Center),
                ]
                .align_x(Horizontal::Left),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style::default().background(Color::BLACK.scale_alpha(0.8)))
        }
    }
}
