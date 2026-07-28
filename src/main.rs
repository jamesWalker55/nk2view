mod nk2;
mod widgets;

use iced::futures::channel::mpsc::UnboundedSender;
use iced::widget::canvas::Canvas;
use iced::widget::{column, container, text};
use iced::{Element, Length, Task, alignment};

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, StreamExt};

use crate::nk2::eventloop::{KBEvent, spawn_event_thread};
use crate::nk2::scene::Scene;
use crate::widgets::keyboard::KeyboardProgram;

pub fn main() -> iced::Result {
    iced::application(boot, update, view)
        .subscription(subscription)
        .antialiasing(true)
        .title("Live MIDI Keyboard Visualizer")
        .run()
}

struct App {
    cmd_tx: UnboundedSender<Vec<u8>>,
    state: State,
}

#[derive(Debug)]
enum State {
    Connected(ConnectedState),
    Disconnected(DisconnectedState),
}

#[derive(Debug)]
struct ConnectedState {
    scene: Scene,
    pressed_keys: [bool; 128],
    popup: Option<String>,
}

#[derive(Debug)]
struct DisconnectedState {
    message: String,
}

#[derive(Debug)]
enum Message {
    Initialized { cmd_tx: UnboundedSender<Vec<u8>> },
    KBEvent(KBEvent),
    RootNoteChanged(u8), // Emitted when the user clicks a key
}

fn boot() -> (Option<App>, Task<Message>) {
    (None, Task::none())
}

fn update(app: &mut Option<App>, msg: Message) -> Task<Message> {
    let Some(app) = app else {
        // app not yet initialized
        if let Message::Initialized { cmd_tx } = msg {
            let new_state = DisconnectedState {
                message: "just started".into(),
            };
            *app = Some(App {
                cmd_tx,
                state: State::Disconnected(new_state),
            })
        }
        return Task::none();
    };

    match app.state {
        // disconnected from keyboard
        State::Disconnected(ref mut state) => match msg {
            Message::KBEvent(KBEvent::ConnectionEstablished(scene)) => {
                app.state = State::Connected(ConnectedState {
                    scene,
                    pressed_keys: [false; _],
                    popup: None,
                })
            }
            Message::KBEvent(KBEvent::ConnectionError(text)) => {
                state.message = text;
            }
            _ => {
                println!("got `{msg:?}` while disconnected?!");
            }
        },

        // connected to keyboard
        State::Connected(ref mut state) => match msg {
            Message::KBEvent(KBEvent::ConnectionError(text)) => {
                app.state = State::Disconnected(DisconnectedState { message: text });
            }
            Message::KBEvent(KBEvent::ConnectionEstablished(scene)) => {
                state.scene = scene;
            }
            Message::KBEvent(KBEvent::NoteOn(note)) => {
                state.pressed_keys[note as usize] = true;
            }
            Message::KBEvent(KBEvent::NoteOff(note)) => {
                state.pressed_keys[note as usize] = false;
            }
            Message::KBEvent(KBEvent::AllNotesOff) => {
                for key in state.pressed_keys.iter_mut() {
                    *key = false;
                }
            }
            Message::KBEvent(KBEvent::SceneUpdated(scene)) => {
                state.scene = scene;
            }
            Message::KBEvent(KBEvent::Ack(ack)) => match ack {
                nk2::msg::Ack::LoadCompleted(ch) => {
                    state.popup = Some(format!("Load Completed (ch. {ch})"));
                    println!("TODO: Ack::LoadCompleted({ch})");
                }
                nk2::msg::Ack::WriteCompleted(ch) => {
                    state.popup = Some(format!("Write Completed (ch. {ch})"));
                    println!("TODO: Ack::WriteCompleted({ch})");
                }
                nk2::msg::Ack::LoadError(ch) => {
                    state.popup = Some(format!("Load Error (ch. {ch})"));
                    println!("TODO: Ack::LoadError({ch})")
                }
                nk2::msg::Ack::WriteError(ch) => {
                    state.popup = Some(format!("Write Error (ch. {ch})"));
                    println!("TODO: Ack::WriteError({ch})")
                }
            },
            Message::RootNoteChanged(new_root) => {
                println!("Root note changed to: {}", new_root);
                state.scene.transpose = new_root;

                let req =
                    crate::nk2::msg::load_scene_request(state.scene.midi_channel, &state.scene);
                println!("send load scene request: {req:?}");
                app.cmd_tx
                    .unbounded_send(req.into())
                    .expect("TODO: midi worker terminated unexpectedly");
            }
            Message::Initialized { cmd_tx: _ } => {
                unreachable!("should not receive Initialized message")
            }
        },
    }

    Task::none()
}

fn view(app: &Option<App>) -> Element<'_, Message> {
    let Some(app) = app else {
        return container(text("initializing").align_x(alignment::Alignment::Center))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    };

    let canvas = Canvas::new(KeyboardProgram {
        pressed_keys: match app.state {
            State::Connected(ref state) => &state.pressed_keys,
            State::Disconnected(_) => &const { [false; _] },
        },
        root_note: match app.state {
            State::Connected(ref state) => state.scene.transpose,
            State::Disconnected(_) => 64,
        },
        on_root_note_changed: Box::new(Message::RootNoteChanged),
    })
    .width(Length::Fill)
    .height(Length::Fixed(150.0));

    if matches!(app.state, State::Disconnected(_)) {
        return text("disconnected").into();
    }

    container(
        column![text("Live MIDI Keyboard Visualizer").size(30), canvas]
            .spacing(30)
            .align_x(alignment::Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn subscription(_: &Option<App>) -> iced::Subscription<Message> {
    iced::Subscription::run(|| {
        iced::stream::channel(100, |mut output: mpsc::Sender<Message>| async move {
            let (cmd_tx, mut event_rx) = spawn_event_thread();

            output
                .send(Message::Initialized { cmd_tx })
                .await
                .expect("initialize - send cmd_tx");

            while let Some(evt) = event_rx.next().await {
                let _ = output.send(Message::KBEvent(evt)).await;
            }

            std::future::pending().await
        })
    })
}
