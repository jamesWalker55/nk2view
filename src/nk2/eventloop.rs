use std::time::Duration;

use iced::futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use midi_control::MidiMessage;
use smol::future::FutureExt as _;

use crate::nk2::{
    connection::{create_input_connection, create_output_connection},
    msg,
    scene::Scene,
};

/// How long to wait before establishing/retrying a new connection
const RETRY_DURATION: Duration = Duration::from_millis(500);

/// How long between "fetch scene" requests to the keyboard
const PING_DURATION: Duration = Duration::from_millis(500);

/// Limited subset of MIDI events
#[derive(Debug, Clone)]
pub enum KBEvent {
    // messages from keyboard
    NoteOn(u8),
    NoteOff(u8),
    AllNotesOff,
    SceneUpdated(Scene),
    Ack(msg::Ack),
    // messages from establishing connection with keyboard
    ConnectionEstablished(Scene),
    ConnectionError(String),
}

impl KBEvent {
    fn from_midi_message(msg: &MidiMessage) -> Option<Self> {
        match msg {
            MidiMessage::NoteOn(_ch, evt) => Some(KBEvent::NoteOn(evt.key)),
            MidiMessage::NoteOff(_ch, evt) => Some(KBEvent::NoteOff(evt.key)),
            MidiMessage::ControlChange(_ch, evt) => {
                if evt.control == 120 || evt.control == 123 {
                    Some(KBEvent::AllNotesOff)
                } else {
                    None
                }
            }
            MidiMessage::SysEx(evt) => {
                if let Ok(evt) = msg::Ack::parse_sysex(evt) {
                    Some(KBEvent::Ack(evt))
                } else if let Ok(evt) = msg::SceneDump::parse_sysex(evt) {
                    Some(KBEvent::SceneUpdated(evt.1))
                } else {
                    // TODO: handle more sysex events
                    None
                }
            }
            // ignore all other messages
            _ => None,
        }
    }
}

enum SessionError {
    ConnectionLost(String),
    MainThreadDropped,
}

pub fn spawn_event_thread() -> (UnboundedSender<Vec<u8>>, UnboundedReceiver<KBEvent>) {
    let (simple_tx, simple_rx) = mpsc::unbounded::<KBEvent>();
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded::<Vec<u8>>();

    std::thread::spawn(move || {
        smol::block_on(async {
            // run forever until main thread drops the receiver
            loop {
                match run_session(&simple_tx, &mut cmd_rx).await {
                    // it will never return Ok, I'm just using `Result` so I can use `?` inside the function
                    Ok(_) => unreachable!("session loop should never exit cleanly"),

                    Err(SessionError::MainThreadDropped) => {
                        // The main application closed the receiver channel, stop the thread
                        break;
                    }

                    Err(SessionError::ConnectionLost(err_msg)) => {
                        // keyboard disconnected / failed to connect
                        // emit error and retry
                        let evt = KBEvent::ConnectionError(err_msg);
                        if simple_tx.unbounded_send(evt).is_err() {
                            // main thread dropped the receiver, quit this thread
                            break;
                        }

                        // wait a bit before trying again
                        smol::Timer::after(RETRY_DURATION).await;
                    }
                }
            }
        });
    });

    (cmd_tx, simple_rx)
}

/// Run an (almost) infinite loop that:
///
/// 1. connects to MIDI keyboard
/// 2. determine keyboard channel
/// 3. infinitely send "simple events" to `simple_tx`
///
/// This function will stop looping if:
///
/// - any of the above steps fail
/// - keyboard unexpectedly disconnects on step 3
///
/// TODO: Handle when channel changes
async fn run_session(
    simple_tx: &UnboundedSender<KBEvent>,
    cmd_rx: &mut UnboundedReceiver<Vec<u8>>,
) -> Result<(), SessionError> {
    // channel for forwarding events from MIDI worker to this thread
    let (midi_tx, mut midi_rx) = mpsc::unbounded::<MidiMessage>();

    // create MIDI input, forwarding events into this scope
    // keep `_midi_in` alive to keep connection alive
    let _midi_in = create_input_connection(
        move |_stamp, message, tx| {
            let _ = tx.unbounded_send(MidiMessage::from(message));
        },
        midi_tx,
    )
    .map_err(|e| SessionError::ConnectionLost(e.to_string()))?;

    // create MIDI output
    let mut midi_out =
        create_output_connection().map_err(|e| SessionError::ConnectionLost(e.to_string()))?;

    // determine what channel the keyboard is on
    let dump = {
        // request keyboard to dump scene on every channel
        for i in 0u8..=15u8 {
            let data: Vec<u8> = msg::dump_scene_request(i).into();
            midi_out
                .send(&data)
                .map_err(|e| SessionError::ConnectionLost(e.to_string()))?;
        }

        // must receive message from keyboard within 50ms
        let timeout_task = async {
            smol::Timer::after(Duration::from_millis(50)).await;
            Err(SessionError::ConnectionLost(
                "timeout trying to determine the keyboard channel".into(),
            ))
        };

        // wait for the first scene update message
        let fetch_task = async {
            // Using .recv() and ignoring irrelevant messages gracefully
            while let Ok(msg) = midi_rx.recv().await {
                if let MidiMessage::SysEx(sysex) = msg {
                    if let Ok(dump) = msg::SceneDump::parse_sysex(&sysex) {
                        return Ok(dump);
                    } else {
                        // TODO: log error
                    }
                }
            }
            Err(SessionError::ConnectionLost(
                "channel closed while fetching scene".into(),
            ))
        };

        fetch_task.or(timeout_task).await
    }?;

    // emit success signal
    simple_tx
        .unbounded_send(KBEvent::ConnectionEstablished(dump.1))
        .map_err(|_| SessionError::MainThreadDropped)?;

    // keyboard ping loop + send simple events
    let mut ping_timer = smol::Timer::after(PING_DURATION);

    loop {
        enum LoopAction {
            MidiIn(Option<MidiMessage>),
            CommandIn(Option<Vec<u8>>),
            Ping,
        }

        // receive keyboard event
        let rx_task = async {
            match midi_rx.recv().await {
                Ok(msg) => LoopAction::MidiIn(Some(msg)),
                Err(_) => LoopAction::MidiIn(None),
            }
        };

        // send keyboard event
        let cmd_task = async {
            match cmd_rx.recv().await {
                Ok(cmd) => LoopAction::CommandIn(Some(cmd)),
                Err(_) => LoopAction::CommandIn(None),
            }
        };

        // keyboard ping timer
        let ping_task = async {
            (&mut ping_timer).await;
            LoopAction::Ping
        };

        match rx_task.or(cmd_task).or(ping_task).await {
            // receive keyboard event
            LoopAction::MidiIn(Some(msg)) => {
                if let Some(evt) = KBEvent::from_midi_message(&msg)
                    && simple_tx.unbounded_send(evt).is_err()
                {
                    return Err(SessionError::MainThreadDropped);
                }
            }
            LoopAction::MidiIn(None) => {
                return Err(SessionError::ConnectionLost(
                    "MIDI worker ended unexpectedly".into(),
                ));
            }

            // send keyboard event
            LoopAction::CommandIn(Some(cmd)) => {
                midi_out
                    .send(&cmd)
                    .map_err(|e| SessionError::ConnectionLost(e.to_string()))?;
            }
            LoopAction::CommandIn(None) => {
                // If the main thread drops the sender, it means the application is likely shutting down.
                return Err(SessionError::MainThreadDropped);
            }

            // keyboard ping timer
            LoopAction::Ping => {
                ping_timer = smol::Timer::after(PING_DURATION);

                let req: Vec<u8> = msg::dump_scene_request(dump.0).into();
                midi_out
                    .send(&req)
                    .map_err(|e| SessionError::ConnectionLost(e.to_string()))?;
            }
        }
    }
}

#[cfg(test)]
#[test]
#[ignore = "needs keyboard, runs forever, might become unkillable process"]
// run with --nocapture
fn test_session() {
    smol::block_on(async {
        let (_cmd_tx, mut events) = spawn_event_thread();

        while let Ok(evt) = events.recv().await {
            println!("{evt:?}");
        }
    });
}

#[cfg(test)]
#[test]
#[ignore = "needs keyboard"]
fn test_session_2() {
    smol::block_on(async {
        let (cmd_tx, mut events) = spawn_event_thread();

        cmd_tx.unbounded_send(msg::dump_scene_request(0).into());

        // let evt = events.recv().await;
        // dbg!(evt);
        while let Ok(evt) = events.recv().await {
            println!("{evt:?}");
        }
    });
}
