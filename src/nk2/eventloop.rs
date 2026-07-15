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
pub enum SimpleEvent {
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

impl SimpleEvent {
    fn from_midi_message(msg: &MidiMessage) -> Option<Self> {
        match msg {
            MidiMessage::NoteOn(_ch, evt) => Some(SimpleEvent::NoteOn(evt.key)),
            MidiMessage::NoteOff(_ch, evt) => Some(SimpleEvent::NoteOff(evt.key)),
            MidiMessage::ControlChange(_ch, evt) => {
                if evt.control == 120 || evt.control == 123 {
                    Some(SimpleEvent::AllNotesOff)
                } else {
                    None
                }
            }
            MidiMessage::SysEx(evt) => {
                if let Ok(evt) = msg::Ack::parse_sysex(evt) {
                    Some(SimpleEvent::Ack(evt))
                } else if let Ok(evt) = msg::SceneDump::parse_sysex(evt) {
                    Some(SimpleEvent::SceneUpdated(evt.1))
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

pub fn spawn_event_thread() -> UnboundedReceiver<SimpleEvent> {
    let (simple_tx, simple_rx) = mpsc::unbounded::<SimpleEvent>();

    std::thread::spawn(move || {
        smol::block_on(async {
            // run forever until main thread drops the receiver
            loop {
                match run_session(&simple_tx).await {
                    // it will never return Ok, I'm just using `Result` so I can use `?` inside the function
                    Ok(_) => unreachable!("session loop should never exit cleanly"),

                    Err(SessionError::MainThreadDropped) => {
                        // The main application closed the receiver channel, stop the thread
                        break;
                    }

                    Err(SessionError::ConnectionLost(err_msg)) => {
                        // keyboard disconnected / failed to connect
                        // emit error and retry
                        let evt = SimpleEvent::ConnectionError(err_msg);
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

    simple_rx
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
async fn run_session(simple_tx: &UnboundedSender<SimpleEvent>) -> Result<(), SessionError> {
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

        fetch_task.or(timeout_task).await?
    };

    // emit success signal
    simple_tx
        .unbounded_send(SimpleEvent::ConnectionEstablished(dump.1))
        .map_err(|_| SessionError::MainThreadDropped)?;

    // keyboard ping loop + send simple events
    let mut ping_timer = smol::Timer::after(PING_DURATION);

    loop {
        let ping_task = async {
            (&mut ping_timer).await;
            None // Signifies timeout
        };

        let rx_task = async {
            Some(midi_rx.recv().await) // Signifies channel event
        };

        match rx_task.or(ping_task).await {
            // incoming event
            Some(Ok(msg)) => {
                // Using let_chains here perfectly!
                if let Some(evt) = SimpleEvent::from_midi_message(&msg)
                    && simple_tx.unbounded_send(evt).is_err()
                {
                    return Err(SessionError::MainThreadDropped);
                }
            }
            // MIDI worker tx got dropped, something went wrong with MIDI worker
            Some(Err(_)) => {
                return Err(SessionError::ConnectionLost(
                    "MIDI worker ended unexpectedly".into(),
                ));
            }
            // it's keyboard pinging time
            None => {
                ping_timer = smol::Timer::after(PING_DURATION); // Reset timer

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
#[ignore = "needs keyboard, runs forever"]
fn test_session() {
    smol::block_on(async {
        let mut events = spawn_event_thread();
        while let Ok(evt) = events.recv().await {
            println!("{evt:?}");
        }
    });
}
