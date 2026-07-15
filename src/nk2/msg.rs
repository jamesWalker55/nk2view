use midi_control::{MidiMessage, SysExEvent, message::SysExType, sysex::ManufacturerId};
use thiserror::Error;

use crate::nk2::scene::{InvalidSceneParam, Scene};

/// `ch` must match the keyboard's channel
pub fn dump_scene_request(ch: u8) -> MidiMessage {
    MidiMessage::SysEx(SysExEvent::new_manufacturer(
        ManufacturerId::Id(0x42),
        &[
            0x40 + (ch & 0x0f),
            0x00,
            0x01,
            0x11,
            0x01,
            0x1F,
            0x10, // dump request
            0x00,
            0xF7,
        ],
    ))
}

/// `ch` must match the keyboard's channel
pub fn save_scene_request(ch: u8) -> MidiMessage {
    MidiMessage::SysEx(SysExEvent::new_manufacturer(
        ManufacturerId::Id(0x42),
        &[
            0x40 + (ch & 0x0f),
            0x00,
            0x01,
            0x11,
            0x01,
            0x1F,
            0x11, // write request
            0x00,
            0xF7,
        ],
    ))
}

/// `ch` must match the keyboard's channel
pub fn restore_scene_request(ch: u8) -> MidiMessage {
    MidiMessage::SysEx(SysExEvent::new_manufacturer(
        ManufacturerId::Id(0x42),
        &[
            0x40 + (ch & 0x0f),
            0x00,
            0x01,
            0x11,
            0x01,
            0x1F,
            0x14, // change request
            0x00,
            0xF7,
        ],
    ))
}

/// `ch` must match the keyboard's channel
pub fn load_scene_request(ch: u8, scene: &Scene) -> MidiMessage {
    let mut data = Vec::from(&[
        0x40 + (ch & 0x0f),
        0x00,
        0x01,
        0x11,
        0x01,
        0x4B,
        0x40, // change request
    ]);
    data.extend_from_slice(&scene.to_midi_bytes());
    data.push(0xF7); // EOX

    MidiMessage::SysEx(SysExEvent {
        r#type: SysExType::Manufacturer(ManufacturerId::Id(0x42)),
        data,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneDump(pub u8, pub Scene);

#[derive(Error, Debug)]
pub enum ParseSceneDumpError {
    #[error("not a scene dump event")]
    NotDump,
    #[error("scene dump event is malformed")]
    Malformed,
    #[error("scene dump event contains invalid scene")]
    InvalidScene(#[from] InvalidSceneParam),
}

impl SceneDump {
    pub fn parse_sysex(evt: &SysExEvent) -> Result<SceneDump, ParseSceneDumpError> {
        if !matches!(
            evt.r#type,
            SysExType::Manufacturer(ManufacturerId::Id(0x42))
        ) {
            return Err(ParseSceneDumpError::NotDump);
        }

        let Ok(data): Result<&[u8; 83], _> = evt.data.as_slice().try_into() else {
            return Err(ParseSceneDumpError::Malformed);
        };

        let channel = {
            if data[0] & 0xF0 != 0x40 {
                return Err(ParseSceneDumpError::Malformed);
            }
            data[0] & 0x0F
        };

        // preamble
        if data[1..8] != [0x00, 0x01, 0x11, 0x01, 0x7F, 0x4b, 0x40] {
            return Err(ParseSceneDumpError::Malformed);
        }

        // ending
        if data[82] != 0xF7 {
            return Err(ParseSceneDumpError::Malformed);
        }

        let scene_data: &[u8; 74] = &data[8..82].try_into().expect("should be same length");
        let scene = Scene::from_midi_bytes(scene_data)?;

        Ok(SceneDump(channel, scene))
    }
}

#[cfg(test)]
#[test]
fn test_scene_dump() {
    use crate::nk2::scene::{ButtonBehaviour, Speed, VelocityCurve};

    let evt = SysExEvent {
        r#type: SysExType::Manufacturer(ManufacturerId::Id(66)),
        data: vec![
            64, 0, 1, 17, 1, 127, 75, 64, 122, 0, 127, 2, 127, 127, 127, 127, 113, 127, 66, 1, 100,
            127, 127, 127, 3, 127, 127, 1, 1, 0, 0, 127, 6, 2, 127, 127, 1, 64, 0, 0, 124, 127, 2,
            127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127,
            127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127,
            127, 127, 127, 1, 127, 247,
        ],
    };
    let msg = SceneDump::parse_sysex(&evt).unwrap();
    let expected = SceneDump(
        0,
        Scene {
            midi_channel: 0,
            pitch_bend_speed: Speed::Normal,
            transpose: 66,
            velocity_curve: VelocityCurve::Normal,
            velocity_constant_value: 100,
            mod_enable: true,
            mod_cc: 1,
            mod_behaviour: ButtonBehaviour::Momentary,
            mod_off_value: 0,
            mod_on_value: 127,
            mod_speed: Speed::Normal,
            sustain_enable: true,
            sustain_cc: 64,
            sustain_behaviour: ButtonBehaviour::Momentary,
            sustain_off_value: 0,
            sustain_on_value: 127,
            sustain_speed: Speed::Normal,
        },
    );
    assert_eq!(msg, expected);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ack {
    LoadCompleted(u8),
    LoadError(u8),
    WriteCompleted(u8),
    WriteError(u8),
}

#[derive(Error, Debug)]
pub enum ParseAckError {
    #[error("not an ack event")]
    NotAck,
    #[error("ack event is malformed")]
    Malformed,
    #[error("unknown ack event type {0}")]
    Unknown(u8),
}

impl Ack {
    pub fn parse_sysex(evt: &SysExEvent) -> Result<Ack, ParseAckError> {
        if !matches!(
            evt.r#type,
            SysExType::Manufacturer(ManufacturerId::Id(0x42))
        ) {
            return Err(ParseAckError::NotAck);
        }

        let Ok(data): Result<&[u8; 9], _> = evt.data.as_slice().try_into() else {
            return Err(ParseAckError::NotAck);
        };

        let channel = {
            if data[0] & 0xF0 != 0x40 {
                return Err(ParseAckError::Malformed);
            }
            data[0] & 0x0F
        };

        // preamble
        if data[1..6] != [0x00, 0x01, 0x11, 0x01, 0x5F] {
            return Err(ParseAckError::Malformed);
        }

        // ending
        if data[7..9] != [0x00, 0xF7] {
            return Err(ParseAckError::Malformed);
        }

        match data[6] {
            0x23 => Ok(Ack::LoadCompleted(channel)),
            0x24 => Ok(Ack::LoadError(channel)),
            0x21 => Ok(Ack::WriteCompleted(channel)),
            0x22 => Ok(Ack::WriteError(channel)),
            // TODO: unknown command type
            x => Err(ParseAckError::Unknown(x)),
        }
    }
}
