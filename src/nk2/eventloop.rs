use std::{
    sync::{Arc, atomic::AtomicBool},
    thread,
    time::Duration,
};

use iced::futures::{
    FutureExt,
    channel::mpsc::{self, UnboundedReceiver},
};
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

                // create MIDI input, forwarding events into this scope
                // keep `_midi_in` alive to keep connection alive
                let _midi_in = match create_input_connection(
                    move |stamp, message, tx| {
                        let msg = MidiMessage::from(message);
                        // don't care about error here.
                        //
                        // the only error that happens is when rx gets dropped, and that is already handled
                        // below in the ping loop.
                        let _ = tx.unbounded_send(msg);
                    },
                    midi_tx,
                ) {
                    Ok(x) => x,
                    Err(err) => {
                        // emit error event to channel
                        let evt = SimpleEvent::ConnectionError(err.to_string());
                        if simple_tx.unbounded_send(evt).is_err() {
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
                        if simple_tx.unbounded_send(evt).is_err() {
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
                            if simple_tx.unbounded_send(evt).is_err() {
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
                        while let Ok(msg) = midi_rx.recv().await {
                            if let MidiMessage::SysEx(sysex) = msg {
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
                        }

                        Err("channel closed while trying to determine the channel of the keyboard")
                    };

                    match fetch_task.or(timeout_task).await {
                        Ok(x) => x,
                        Err(err) => {
                            // emit error event to channel
                            let evt = SimpleEvent::ConnectionError(err.to_string());
                            if simple_tx.unbounded_send(evt).is_err() {
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
                    if simple_tx.unbounded_send(evt).is_err() {
                        // if failed to emit error, `rx` has been dropped
                        break 'outer;
                    }
                }

                // keyboard ping loop + send simple events
                let mut ping_timer = smol::Timer::after(PING_DURATION);

                loop {
                    // Some = rx event
                    // None = timeout ping
                    let ping_task = async {
                        (&mut ping_timer).await;
                        None
                    };
                    let rx_task = async { Some(midi_rx.recv().await) };

                    match rx_task.or(ping_task).await {
                        // incoming event
                        Some(Ok(msg)) => {
                            if let Some(evt) = SimpleEvent::from_midi_message(&msg) {
                                if simple_tx.unbounded_send(evt).is_err() {
                                    // if failed to emit error, `rx` has been dropped
                                    break 'outer;
                                }
                            }
                        }
                        // MIDI worker tx got dropped, something went wrong with MIDI worker
                        Some(Err(_)) => {
                            // `midi_rx` stream ended (MIDI input callback dropped)
                            let evt = SimpleEvent::ConnectionError(
                                "MIDI worker ended unexpectedly".into(),
                            );
                            if simple_tx.unbounded_send(evt).is_err() {
                                // if failed to emit error, `rx` has been dropped
                                break 'outer;
                            }

                            // restart this loop and try to connect again
                            thread::sleep(RETRY_DURATION);
                            continue 'outer;
                        }
                        // it's keyboard pinging time
                        None => {
                            // reset timer for next loops
                            ping_timer = smol::Timer::after(PING_DURATION);

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
                }
            }
        });
    });

    simple_rx
}
