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
const RETRY_DURATION: Duration = Duration::from_millis(100);

/// Limited subset of MIDI events from keyboard to client
#[derive(Debug, Clone)]
pub enum KBEvent {
    // messages from keyboard
    NoteOn(u8),
    NoteOff(u8),
    AllNotesOff,
    SceneDump(Scene),
    Ack(msg::Ack),
    // messages from establishing connection with keyboard
    ConnectionEstablished,
    ConnectionLost(String),
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
                    Some(KBEvent::SceneDump(evt.1))
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

/// Actions we can perform to the keyboard
#[derive(Debug)]
pub enum KBAction {
    Reconnect,
    Send(MidiMessage),
}

/// Internal enum for handling control flow
#[derive(Debug, Clone)]
enum SessionError {
    ConnectionLost(String),
    MainThreadDropped,
}

pub fn spawn_event_thread() -> (UnboundedSender<KBAction>, UnboundedReceiver<KBEvent>) {
    let (evt_tx, evt_rx) = mpsc::unbounded::<KBEvent>();
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded::<KBAction>();

    std::thread::spawn(move || {
        smol::block_on(async {
            // run forever until main thread drops the receiver
            loop {
                match run_session(&evt_tx, &mut cmd_rx).await {
                    Err(SessionError::MainThreadDropped) => {
                        // The main application closed the receiver channel, stop the thread
                        break;
                    }

                    // return Ok(()) to indicate a user-requested refresh
                    Ok(_) => {
                        let evt = KBEvent::ConnectionLost("user-triggered refresh".into());
                        if evt_tx.unbounded_send(evt).is_err() {
                            // main thread dropped the receiver, quit this thread
                            break;
                        }

                        continue;
                    }

                    Err(SessionError::ConnectionLost(err_msg)) => {
                        // keyboard disconnected / failed to connect
                        // emit error and retry
                        let evt = KBEvent::ConnectionLost(err_msg);
                        if evt_tx.unbounded_send(evt).is_err() {
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

    (cmd_tx, evt_rx)
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
    cmd_rx: &mut UnboundedReceiver<KBAction>,
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

    // emit success signal
    simple_tx
        .unbounded_send(KBEvent::ConnectionEstablished)
        .map_err(|_| SessionError::MainThreadDropped)?;

    loop {
        enum LoopAction {
            MidiIn(MidiMessage),
            MidiWorkerDied,
            PerformAction(KBAction),
            MainThreadDropped,
        }

        // receive keyboard event
        let rx_task = async {
            match midi_rx.recv().await {
                Ok(msg) => LoopAction::MidiIn(msg),
                Err(_) => LoopAction::MidiWorkerDied,
            }
        };

        // send keyboard event
        let cmd_task = async {
            match cmd_rx.recv().await {
                Ok(cmd) => LoopAction::PerformAction(cmd),
                Err(_) => LoopAction::MainThreadDropped,
            }
        };

        match rx_task.or(cmd_task).await {
            // receive keyboard event
            LoopAction::MidiIn(msg) => {
                if let Some(evt) = KBEvent::from_midi_message(&msg)
                    && simple_tx.unbounded_send(evt).is_err()
                {
                    return Err(SessionError::MainThreadDropped);
                }
            }
            LoopAction::MidiWorkerDied => {
                return Err(SessionError::ConnectionLost(
                    "MIDI worker ended unexpectedly".into(),
                ));
            }

            // send keyboard event
            LoopAction::PerformAction(KBAction::Send(msg)) => {
                let msg: Vec<u8> = msg.into();
                midi_out
                    .send(&msg)
                    .map_err(|e| SessionError::ConnectionLost(e.to_string()))?;
            }
            LoopAction::PerformAction(KBAction::Reconnect) => {
                // manually trigger reconnect
                return Ok(());
            }
            LoopAction::MainThreadDropped => {
                // If the main thread drops the sender, it means the application is likely shutting down.
                return Err(SessionError::MainThreadDropped);
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

        cmd_tx.unbounded_send(KBAction::Send(msg::dump_scene_request(0)));

        // let evt = events.recv().await;
        // dbg!(evt);
        while let Ok(evt) = events.recv().await {
            println!("{evt:?}");
        }
    });
}
