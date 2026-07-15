use std::{
    sync::{Arc, atomic::AtomicBool},
    thread,
    time::Duration,
};

use iced::futures::channel::mpsc::{self, UnboundedReceiver};
use midi_control::MidiMessage;
use smol::future::FutureExt as _;

use crate::nk2::{
    connection::{create_input_connection, create_output_connection},
    msg,
    scene::Scene,
};

/// How long to wait before establishing/retrying a new connection
const RETRY_DURATION: Duration = Duration::from_millis(200);

/// How long between "fetch scene" requests to the keyboard
const PING_DURATION: Duration = Duration::from_millis(200);

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
            MidiMessage::NoteOn(ch, evt) => Some(SimpleEvent::NoteOn(evt.key)),
            MidiMessage::NoteOff(ch, evt) => Some(SimpleEvent::NoteOff(evt.key)),
            MidiMessage::ControlChange(ch, evt) => {
                if evt.control == 120 || evt.control == 123 {
                    Some(SimpleEvent::AllNotesOff)
                } else {
                    None
                }
            }
            MidiMessage::SysEx(evt) => {
                if let Ok(evt) = msg::Ack::parse_sysex(&evt) {
                    Some(SimpleEvent::Ack(evt))
                } else if let Ok(evt) = msg::SceneDump::parse_sysex(&evt) {
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

pub fn spawn_event_thread() -> UnboundedReceiver<SimpleEvent> {
    // channel for communicating with main thread
    let (simple_tx, mut simple_rx) = mpsc::unbounded::<SimpleEvent>();

    std::thread::spawn(move || {
        smol::block_on(async {
            // connection restart loop, break this loop to stop the thread
            'outer: loop {
                // channel for forwarding events from MIDI worker to this thread
                let (midi_tx, mut midi_rx) = mpsc::unbounded::<MidiMessage>();
                let should_exit = Arc::new(AtomicBool::new(false));

                // create MIDI input, forwarding events into this scope
                // keep `_midi_in` alive to keep connection alive
                let _midi_in = match create_input_connection(
                    move |stamp, message, (tx, should_exit)| {
                        let msg = MidiMessage::from(message);
                        if let Err(err) = tx.unbounded_send(msg) {
                            // if failed to send, `rx` has been dropped
                            // rx is dropped, the program must have quit
                            let _ = should_exit.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    },
                    (midi_tx.clone(), should_exit.clone()),
                ) {
                    Ok(x) => x,
                    Err(err) => {
                        // emit error event to channel
                        let evt = SimpleEvent::ConnectionError(err.to_string());
                        if let Err(_) = simple_tx.unbounded_send(evt) {
                            // if failed to emit error, `rx` has been dropped
                            break 'outer;
                        }

                        // restart this loop and try to connect again
                        thread::sleep(RETRY_DURATION);
                        continue 'outer;
                    }
                };

                // create MIDI output
                let mut midi_out = match create_output_connection() {
                    Ok(x) => x,
                    Err(err) => {
                        // emit error event to channel
                        let evt = SimpleEvent::ConnectionError(err.to_string());
                        if let Err(_) = simple_tx.unbounded_send(evt) {
                            // if failed to emit error, `rx` has been dropped
                            break 'outer;
                        }

                        // restart this loop and try to connect again
                        thread::sleep(RETRY_DURATION);
                        continue 'outer;
                    }
                };

                // determine what channel the keyboard is on
                let dump = {
                    // request keyboard to dump scene on every channel
                    for i in 0u8..=15u8 {
                        let data: Vec<u8> = msg::dump_scene_request(i).into();
                        let res = midi_out.send(&data);

                        // if failed to send request...
                        if let Err(err) = res {
                            // emit error event to channel
                            let evt = SimpleEvent::ConnectionError(err.to_string());
                            if let Err(_) = simple_tx.unbounded_send(evt) {
                                // if failed to emit error, `rx` has been dropped
                                break 'outer;
                            }

                            // restart this loop and try to connect again
                            thread::sleep(RETRY_DURATION);
                            continue 'outer;
                        }
                    }

                    // must receive message from keyboard within 50ms
                    let timeout_task = async {
                        smol::Timer::after(Duration::from_millis(50)).await;
                        Err("timeout trying to determine the channel of the keyboard")
                    };

                    // wait for the first scene update message
                    let fetch_task = async {
                        while let Ok(MidiMessage::SysEx(sysex)) = midi_rx.recv().await {
                            match msg::SceneDump::parse_sysex(&sysex) {
                                Ok(dump) => {
                                    return Ok(dump);
                                }
                                Err(err) => {
                                    // TODO: log error
                                    continue;
                                }
                            }
                        }

                        Err("channel closed while trying to determine the channel of the keyboard")
                    };

                    match fetch_task.or(timeout_task).await {
                        Ok(x) => x,
                        Err(err) => {
                            // emit error event to channel
                            let evt = SimpleEvent::ConnectionError(err.to_string());
                            if let Err(_) = simple_tx.unbounded_send(evt) {
                                // if failed to emit error, `rx` has been dropped
                                break 'outer;
                            }

                            // restart this loop and try to connect again
                            thread::sleep(RETRY_DURATION);
                            continue 'outer;
                        }
                    }
                };

                // emit success signal
                {
                    let evt = SimpleEvent::ConnectionEstablished(dump.1);
                    if let Err(_) = simple_tx.unbounded_send(evt) {
                        // if failed to emit error, `rx` has been dropped
                        break 'outer;
                    }
                }

                // keyboard ping loop
                loop {
                    // wait for the ping timer
                    smol::Timer::after(PING_DURATION).await;

                    // check if the MIDI worker has requested exiting
                    if should_exit.load(std::sync::atomic::Ordering::Relaxed) {
                        // exit this thread
                        break 'outer;
                    }

                    // send regular "scene request" ping
                    let req: Vec<u8> = msg::dump_scene_request(dump.0).into();
                    if let Err(err) = midi_out.send(&req) {
                        // emit error event to channel
                        let evt = SimpleEvent::ConnectionError(err.to_string());
                        if let Err(_) = simple_tx.unbounded_send(evt) {
                            // if failed to emit error, `rx` has been dropped
                            break 'outer;
                        }

                        // restart this loop and try to connect again
                        thread::sleep(RETRY_DURATION);
                        continue 'outer;
                    }
                }
            }
        });
    });

    simple_rx
}
