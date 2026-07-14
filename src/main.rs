mod nk2;

use iced::widget::canvas::{self, Canvas, Frame, Path, Program};
use iced::widget::{column, container, text};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Task, Theme, alignment};
use midi_control::MidiMessage;

// Iced conveniently re-exports futures, so we can use its channel to bridge threads!
use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, StreamExt};

pub fn main() -> iced::Result {
    iced::application(boot, update, view)
        .subscription(subscription)
        .antialiasing(true)
        .title("Live MIDI Keyboard Visualizer")
        .run()
}

struct State {
    pressed_keys: [bool; 128],
}

impl Default for State {
    fn default() -> Self {
        Self {
            pressed_keys: [false; 128],
        }
    }
}

#[derive(Debug)]
enum Message {
    // This message is triggered from the Subscription channel
    MidiEventReceived(MidiMessage),
}

fn boot() -> (State, Task<Message>) {
    (State::default(), Task::none())
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::MidiEventReceived(msg) => {
            match msg {
                MidiMessage::NoteOn(channel, evt) => {
                    // Some keyboards send NoteOn with velocity 0 instead of NoteOff
                    if evt.value > 0 {
                        state.pressed_keys[evt.key as usize] = true;
                    } else {
                        state.pressed_keys[evt.key as usize] = false;
                    }
                }
                MidiMessage::NoteOff(channel, evt) => {
                    state.pressed_keys[evt.key as usize] = false;
                }
                _ => (),
            }
        }
    }
    Task::none()
}

fn view(state: &State) -> Element<'_, Message> {
    // Create our Canvas and pass a reference to our state
    let canvas = Canvas::new(KeyboardProgram {
        pressed_keys: &state.pressed_keys,
    })
    .width(Length::Fill)
    .height(Length::Fixed(150.0));

    // A simple UI Layout
    container(
        column![text("Live MIDI Keyboard Visualizer").size(30), canvas]
            .spacing(30)
            .align_x(alignment::Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(40)
    // .center_y()
    .into()
}

// This is where the magic happens!
fn subscription(_state: &State) -> iced::Subscription<Message> {
    iced::Subscription::run(|| {
        iced::stream::channel(
            100, // Iced buffer size
            |mut output: mpsc::Sender<Message>| async move {
                // 1. Create a thread-safe MPSC channel
                let (tx, mut rx) = mpsc::unbounded();

                // 2. Start the midir connection
                let _conn = nk2::connection::create_input_connection(
                    move |_stamp, message, _| {
                        let msg = MidiMessage::from(message);
                        // Push the MIDI message from the midir thread to our async loop
                        let _ = tx.unbounded_send(msg);
                    },
                    (),
                )
                .unwrap();

                // 3. Forward events from the MPSC channel into iced's event loop
                while let Some(msg) = rx.next().await {
                    let _ = output.send(Message::MidiEventReceived(msg)).await;
                }

                // 4. Keep the async task (and thus the _conn variable) alive forever
                std::future::pending().await
            },
        )
    })
}

struct KeyboardProgram<'a> {
    pressed_keys: &'a [bool; 128],
}

impl<'a, Message> Program<Message> for KeyboardProgram<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let key_width = bounds.width / 128.0;

        for i in 0..128 {
            let is_pressed = self.pressed_keys[i];

            if is_pressed {
                // Draw a vibrant red key if pressed
                let color = Color::from_rgb(0.9, 0.2, 0.2);
                let path = Path::rectangle(
                    Point::new(i as f32 * key_width, 0.0),
                    Size::new(key_width.max(1.0), bounds.height),
                );
                frame.fill(&path, color);
            } else {
                // Draw a light grey key with a tiny visual gap if not pressed
                let color = Color::from_rgb(0.9, 0.9, 0.9);
                let path = Path::rectangle(
                    Point::new(i as f32 * key_width, 0.0),
                    Size::new((key_width - 1.0).max(1.0), bounds.height),
                );
                frame.fill(&path, color);
            }
        }

        vec![frame.into_geometry()]
    }
}
