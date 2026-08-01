mod nk2;
mod widgets;

use iced::futures::channel::mpsc::UnboundedSender;
use iced::widget::canvas::Canvas;
use iced::widget::{button, column, container, text};
use iced::{Element, Length, Task, alignment};

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, StreamExt};

use crate::nk2::eventloop::{KBAction, KBEvent, spawn_event_thread};
use crate::nk2::msg::dump_scene_request;
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
    cmd_tx: UnboundedSender<KBAction>,
    state: State,
}

#[derive(Debug)]
enum State {
    /// The main state with the interactable UI etc
    Connected(ConnectedState),
    /// The initial state.
    Disconnected(DisconnectedState),
    /// Transition state between 'disconnected' and 'connected'
    FetchingScene,
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

#[derive(Debug, Clone)]
enum Message {
    Initialized { cmd_tx: UnboundedSender<KBAction> },
    KBEvent(KBEvent),
    RootNoteChanged(u8), // Emitted when the user clicks a key
    ReconnectRequested,  // Emitted when the user clicks the refresh button
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
            Message::KBEvent(KBEvent::ConnectionEstablished) => {
                // send request to fetch scene data
                app.state = State::FetchingScene;

                for i in 0u8..=15u8 {
                    let msg = dump_scene_request(i);
                    app.cmd_tx
                        .unbounded_send(KBAction::Send(msg))
                        .expect("TODO: midi worker terminated unexpectedly");
                }
            }
            Message::KBEvent(KBEvent::ConnectionLost(text)) => {
                state.message = text;
            }
            _ => {
                println!("got `{msg:?}` while disconnected?!");
            }
        },

        // established connection, get some initial data
        State::FetchingScene => match msg {
            Message::KBEvent(KBEvent::SceneDump(scene)) => {
                app.state = State::Connected(ConnectedState {
                    scene,
                    pressed_keys: [false; _],
                    popup: None,
                })
            }
            Message::KBEvent(KBEvent::ConnectionLost(text)) => {
                app.state = State::Disconnected(DisconnectedState { message: text });
            }
            _ => {
                println!("got `{msg:?}` while disconnected?!");
            }
        },

        // connected to keyboard
        State::Connected(ref mut state) => match msg {
            Message::KBEvent(KBEvent::ConnectionLost(text)) => {
                app.state = State::Disconnected(DisconnectedState { message: text });
            }
            Message::KBEvent(KBEvent::ConnectionEstablished) => {
                // should not happen while we are in connected state
                println!("got `KBEvent::ConnectionEstablished` while connected?!");
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
            Message::KBEvent(KBEvent::SceneDump(scene)) => {
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
                    .unbounded_send(KBAction::Send(req))
                    .expect("TODO: midi worker terminated unexpectedly");
            }
            Message::ReconnectRequested => {
                app.cmd_tx
                    .unbounded_send(KBAction::Reconnect)
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
            State::FetchingScene => &const { [false; _] },
        },
        root_note: match app.state {
            State::Connected(ref state) => state.scene.transpose,
            State::Disconnected(_) => 64,
            State::FetchingScene => 64,
        },
        on_root_note_changed: Box::new(Message::RootNoteChanged),
    })
    .width(Length::Fill)
    .height(Length::Fixed(150.0));

    if matches!(app.state, State::Disconnected(_)) {
        return text("disconnected").into();
    }

    container(
        column![
            text("Live MIDI Keyboard Visualizer").size(30),
            canvas,
            button("reconnect").on_press(Message::ReconnectRequested)
        ]
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
