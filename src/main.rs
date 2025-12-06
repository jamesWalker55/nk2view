use std::sync::mpsc::channel;
use std::thread;
use std::time;

use midi_control::consts;
use midi_control::sysex::USysExDecoder;
use midi_control::transport::MidiMessageSend;
use midi_control::vendor::arturia;
use midi_control::{MidiMessage, SysExEvent};

/// Print a message on error returned.
macro_rules! print_on_err {
    ($e:expr) => {
        if let Err(err) = $e {
            eprintln!(
                "{}:{} Error '{}': {}",
                file!(),
                line!(),
                stringify!($e),
                err
            );
        }
    };
}
macro_rules! print_err {
    ($e:expr) => {
        eprintln!("{}:{} Error '{}': {}", file!(), line!(), stringify!($e), $e);
    };
}

/// String to look for when enumerating the MIDI devices
const MIDI_DEVICE_NAME: &str = "nanoKEY2";

fn find_port<T>(midi_io: &T) -> Option<T::Port>
where
    T: midir::MidiIO,
{
    midi_io.ports().into_iter().find(|port| {
        midi_io
            .port_name(port)
            .map(|port_name| port_name == MIDI_DEVICE_NAME)
            .unwrap_or(false)
    })
}

fn main() {
    let (sender, receiver) = channel::<MidiMessage>();

    {
        let midi_input = midir::MidiInput::new("MIDITest").unwrap();
        let Some(device_port) = find_port(&midi_input) else {
            println!("Input device not found!");
            return;
        };

        midi_input
            .connect(
                &device_port,
                MIDI_DEVICE_NAME,
                move |_timestamp: u64, data, sender| {
                    let msg = MidiMessage::from(data);
                    print_on_err!(sender.send(msg));
                },
                sender,
            )
            .expect("failed to create midi input");
    }

    let midi_output = midir::MidiOutput::new("MIDITest").unwrap();
    let Some(device_port) = find_port(&midi_output) else {
        println!("Output device not found!");
        return;
    };

    let mut connect_out = match midi_output.connect(&device_port, MIDI_DEVICE_NAME) {
        Ok(x) => x,
        Err(err) => {
            print_err!(err);
            return;
        }
    };

    let msg = MidiMessage::SysEx(SysExEvent::new_non_realtime(
        consts::usysex::ALL_CALL,
        [0x06, 0x01],
        &[0xf7],
    ));
    print_on_err!(connect_out.send_message(msg));
    println!("Press Control-C at anytime to stop the demo");

    loop {
        if let Ok(msg) = receiver.recv() {
            if let Some(decoder) = USysExDecoder::decode(&msg) {
                if decoder.is_non_realtime()
                    && decoder.target_device() == 0
                    && decoder.general_info_reply_manufacturer_id()
                        == Some(arturia::EXTENDED_ID_VALUE)
                {
                    if decoder.general_info_reply_family() != Some(([2, 0], [4, 2])) {
                        println!("Your device isn't supported by this demo");
                        println!("Only the Arturia minilab MkII is supported");
                    } else {
                        run_demo(&mut connect_out);
                    }
                }
            }
        }

        const WAIT_DURATION: time::Duration = time::Duration::from_millis(100);
        thread::sleep(WAIT_DURATION);
    }
}

fn run_demo(connect_out: &mut midir::MidiOutputConnection) {
    use arturia::v2::param;
    use arturia::v2::{Colour, Control};

    let delay = time::Duration::from_millis(100);

    // pads 1..8
    // for pads 9..16 use 0x78..=0x7f
    let pads = Control::Pad1 as u8..=Control::Pad8 as u8;

    // clear all pads.
    for pad in pads.clone() {
        let msg = arturia::v2::set_value(param::COLOUR as u8, pad, 0x00);
        print_on_err!(connect_out.send_message(msg));
    }

    let colour_set = [
        Colour::Red as u8,
        Colour::Green as u8,
        Colour::Yellow as u8,
        Colour::Blue as u8,
        Colour::Purple as u8,
        Colour::Cyan as u8,
        Colour::White as u8,
    ];
    let mut iter = colour_set.iter();
    loop {
        for pad in pads.clone().rev() {
            let colour = if let Some(c) = iter.next() {
                c
            } else {
                iter = colour_set.iter();
                iter.next().unwrap()
            };
            let msg = arturia::v2::set_value(param::COLOUR as u8, pad, *colour);
            print_on_err!(connect_out.send_message(msg));
            thread::sleep(delay);
        }
        for pad in pads.clone().rev() {
            let msg = arturia::v2::set_value(param::COLOUR as u8, pad, 0);
            print_on_err!(connect_out.send_message(msg));
            thread::sleep(delay);
        }
        for pad in pads.clone() {
            let colour = if let Some(c) = iter.next() {
                c
            } else {
                iter = colour_set.iter();
                iter.next().unwrap()
            };
            let msg = arturia::v2::set_value(param::COLOUR as u8, pad, *colour);
            print_on_err!(connect_out.send_message(msg));
            thread::sleep(delay);
        }
        for pad in pads.clone() {
            let msg = arturia::v2::set_value(param::COLOUR as u8, pad, 0);
            print_on_err!(connect_out.send_message(msg));
            thread::sleep(delay);
        }
    }
}
