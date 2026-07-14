mod nk2;

use iced::Element;
use iced::widget::{button, text};
use iced::window::Level;

#[derive(Default)]
struct State {
    value: u64,
}

#[derive(Debug, Clone)]
enum Message {
    Increment,
}

fn boot() -> State {
    State { value: 0 }
}

fn update(counter: &mut State, message: Message) {
    match message {
        Message::Increment => counter.value += 1,
    }
}

fn view(counter: &State) -> Element<'_, Message> {
    button(text(counter.value))
        .on_press(Message::Increment)
        .into()
}

fn main() -> iced::Result {
    iced::application(boot, update, view)
        .decorations(false)
        .resizable(true)
        // .window({
        //     let mut x = iced::window::Settings::default();
        //     x.decorations = false;
        //     x.resizable = true;
        //     x.platform_specific.undecorated_shadow = true;
        //     // x.icon = todo!();
        //     x
        // })
        .level(Level::AlwaysOnTop)
        .run()
}
