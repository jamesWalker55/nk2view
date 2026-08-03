mod nk2;
mod widgets;

use std::borrow::Cow;
use std::time::Duration;

use iced::alignment::Horizontal;
use iced::futures::channel::mpsc::UnboundedSender;
use iced::widget::canvas::Canvas;
use iced::widget::{button, center, column, container, opaque, row, stack, text};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Task, alignment};

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
        .title("nk2view")
        .window_size([364.0, 90.0])
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
    FetchingScene { _timeout_handle: iced::task::Handle },
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
    Initialized {
        cmd_tx: UnboundedSender<KBAction>,
    },
    /// Emitted if the 'fetching' state doesn't get a response from keyboard within some time
    FetchTimeout,
    KBEvent(KBEvent),
    /// A key is clicked
    RootNoteChanged(u8),
    /// Refresh button pressed
    ReconnectRequested,
    /// Close popup button
    DismissPopup,
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
                for i in 0u8..=15u8 {
                    let msg = dump_scene_request(i);
                    app.cmd_tx
                        .unbounded_send(KBAction::Send(msg))
                        .expect("TODO: midi worker terminated unexpectedly");
                }

                // send timeout message after 50ms if still in 'fetching' state
                let (task, handle) = Task::perform(
                    async { smol::Timer::after(Duration::from_millis(50)).await },
                    |_| Message::FetchTimeout,
                )
                .abortable();

                // transition state
                app.state = State::FetchingScene {
                    _timeout_handle: handle.abort_on_drop(),
                };

                return task;
            }
            Message::KBEvent(KBEvent::ConnectionLost(text)) => {
                state.message = text;
            }
            _ => {
                println!("got `{msg:?}` while disconnected?!");
            }
        },

        // established connection, get some initial data
        State::FetchingScene { .. } => match msg {
            Message::KBEvent(KBEvent::SceneDump(scene)) => {
                app.state = State::Connected(ConnectedState {
                    scene,
                    pressed_keys: [false; _],
                    popup: None,
                })
            }
            Message::FetchTimeout => {
                app.state = State::Disconnected(DisconnectedState {
                    message: "fetch timed out".into(),
                });
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
                nk2::msg::Ack::LoadCompleted(_ch) => (),
                nk2::msg::Ack::WriteCompleted(_ch) => (),
                nk2::msg::Ack::LoadError(ch) => {
                    state.popup = Some(format!("Load Error (ch. {ch})"));
                    let req = crate::nk2::msg::dump_scene_request(ch);
                    app.cmd_tx
                        .unbounded_send(KBAction::Send(req))
                        .expect("TODO: midi worker terminated unexpectedly");
                }
                nk2::msg::Ack::WriteError(ch) => {
                    state.popup = Some(format!("Write Error (ch. {ch})"));
                    let req = crate::nk2::msg::dump_scene_request(ch);
                    app.cmd_tx
                        .unbounded_send(KBAction::Send(req))
                        .expect("TODO: midi worker terminated unexpectedly");
                }
            },
            Message::RootNoteChanged(new_root) => {
                println!("Root note changed to: {}", new_root);
                state.scene.transpose = new_root + 4; // middle C is 60 in MIDI, but 64 in korg

                let req =
                    crate::nk2::msg::load_scene_request(state.scene.midi_channel, &state.scene);
                app.cmd_tx
                    .unbounded_send(KBAction::Send(req))
                    .expect("TODO: midi worker terminated unexpectedly");
            }
            Message::ReconnectRequested => {
                app.cmd_tx
                    .unbounded_send(KBAction::Reconnect)
                    .expect("TODO: midi worker terminated unexpectedly");
            }
            Message::DismissPopup => {
                state.popup = None;
            }
            Message::KBEvent(KBEvent::ConnectionEstablished) => {
                unreachable!("should not receive ConnectionEstablished message")
            }
            Message::Initialized { cmd_tx: _ } => {
                unreachable!("should not receive Initialized message")
            }
            Message::FetchTimeout => unreachable!("should not receive FetchTimeout message"),
        },
    }

    Task::none()
}

fn view(app: &Option<App>) -> Element<'_, Message> {
    // handle `None` state
    let Some(app) = app else {
        return container(text("Initializing...").align_x(alignment::Alignment::Center))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    };

    // the keyboard display
    let canvas = Canvas::new(KeyboardProgram {
        note_width: 20,
        pressed_keys: match app.state {
            State::Connected(ref state) => &state.pressed_keys,
            State::Disconnected(_) => &const { [false; _] },
            State::FetchingScene { .. } => &const { [false; _] },
        },
        root_note: match app.state {
            State::Connected(ref state) => state.scene.transpose - 4,
            State::Disconnected(_) => 60,
            State::FetchingScene { .. } => 60,
        },
        on_note_clicked: Box::new(Message::RootNoteChanged),
    });

    let base = container(
        column![
            canvas.width(Length::Fill).height(Length::Fill),
            widgets::toolbar::toolbar().into(),
        ]
        .align_x(alignment::Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill);

    let popup: Option<(Cow<'static, str>, bool)> = match &app.state {
        State::Connected(state) => state
            .popup
            .as_ref()
            .map(|msg| (Cow::from(msg.clone()), true)),
        State::Disconnected(..) => Some((Cow::from("Connecting to keyboard..."), false)),
        State::FetchingScene { .. } => Some((Cow::from("Fetching scene data..."), false)),
    };

    if let Some((popup_msg, popup_dismissable)) = popup {
        let popup = container(
            (if popup_dismissable {
                column![
                    text(popup_msg),
                    button("Close")
                        .padding([4.0, 8.0])
                        .on_press(Message::DismissPopup)
                ]
                .align_x(Horizontal::Center)
            } else {
                column![text(popup_msg),].align_x(Horizontal::Center)
            })
            .spacing(8.0),
        )
        .padding([4.0, 8.0])
        .style(container::bordered_box);

        let overlay = center(popup)
            .style(|_theme| container::Style::default().background(Color::BLACK.scale_alpha(0.8)));

        stack![base, opaque(overlay)].into()
    } else {
        base.into()
    }
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
                // if received disconnect message, drain any leftover MIDI events from queue
                if matches!(evt, KBEvent::ConnectionLost(_)) {
                    while let Ok(_) = event_rx.try_recv() {
                        // do nothing, just drain
                    }
                }
                let _ = output.send(Message::KBEvent(evt)).await;
            }

            std::future::pending().await
        })
    })
}
