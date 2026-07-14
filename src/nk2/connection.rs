use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput};

use thiserror::Error;

/// Legacy default name in Window API
const NANOKEY2_INPUT_NAME: &str = "nanoKEY2";
/// New name from downloading Windows "MIDI Settings" app and enabling new names
const NANOKEY2_INPUT_NAME_2: &str = "nanoKEY2 _ KEYBOARD";

/// Legacy default name in Window API
const NANOKEY2_OUTPUT_NAME: &str = "nanoKEY2";
/// New name from downloading Windows "MIDI Settings" app and enabling new names
const NANOKEY2_OUTPUT_NAME_2: &str = "nanoKEY2 _ CTRL";

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("failed to initialize midi: {0}")]
    InitMIDI(String),
    #[error("failed to find nanokey2 port")]
    DeviceNotFound,
    #[error("failed to create connection: {0}")]
    Failed(String),
}

pub fn create_input_connection<F, T: Send>(
    callback: F,
    data: T,
) -> Result<MidiInputConnection<T>, ConnectionError>
where
    F: FnMut(u64, &[u8], &mut T) + Send + 'static,
{
    let mut input =
        MidiInput::new("midir input").map_err(|err| ConnectionError::InitMIDI(err.to_string()))?;
    input.ignore(Ignore::None);

    let Some(port) = input.ports().into_iter().find(|port| {
        input
            .port_name(port)
            .map(|port_name| port_name == NANOKEY2_INPUT_NAME || port_name == NANOKEY2_INPUT_NAME_2)
            .unwrap_or(false)
    }) else {
        return Err(ConnectionError::DeviceNotFound);
    };

    input
        .connect(&port, "midir-input", callback, data)
        .map_err(|err| ConnectionError::Failed(err.to_string()))
}

pub fn create_output_connection() -> Result<midir::MidiOutputConnection, ConnectionError> {
    let output = MidiOutput::new("midir output")
        .map_err(|err| ConnectionError::InitMIDI(err.to_string()))?;

    let Some(port) = output.ports().into_iter().find(|port| {
        output
            .port_name(port)
            .map(|port_name| {
                dbg!(&port_name) == NANOKEY2_OUTPUT_NAME || port_name == NANOKEY2_OUTPUT_NAME_2
            })
            .unwrap_or(false)
    }) else {
        return Err(ConnectionError::DeviceNotFound);
    };

    output
        .connect(&port, "midir-output")
        .map_err(|err| ConnectionError::Failed(err.to_string()))
}

#[cfg(test)]
mod tests {
    use midi_control::MidiMessage;
    use midi_control::SysExEvent;
    use std::thread;
    use std::time;

    use super::*;

    #[test]
    #[ignore = "manually run this to test midi messages"]
    fn test_01() {
        let _midi_in = create_input_connection(
            move |stamp, message, _| {
                let msg = MidiMessage::from(message);
                println!("[IN]  {}: {:?}", stamp, msg);
            },
            (),
        )
        .unwrap();
        let mut midi_out = create_output_connection().unwrap();

        loop {
            let msg = MidiMessage::SysEx(SysExEvent::new_non_realtime(
                midi_control::consts::usysex::ALL_CALL,
                [0x06, 0x01],
                &[0xf7],
            ));
            let raw: Vec<u8> = msg.into();
            match midi_out.send(raw.as_slice()) {
                Ok(_) => println!("[OUT] sent!"),
                Err(err) => println!("[OUT] error: {err:?}"),
            }

            const WAIT_DURATION: time::Duration = time::Duration::from_millis(100);
            thread::sleep(WAIT_DURATION);
        }

        // // wait for next enter key press
        // {
        //     println!("Connection open, reading input (press enter to exit) ...");
        //     std::io::stdin().read_line(&mut String::new()).unwrap();
        //     println!("Closing connection");
        // }
    }
}
