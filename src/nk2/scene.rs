use num_enum::{IntoPrimitive, TryFromPrimitive};
use thiserror::Error;

#[derive(Debug, Copy, Clone, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum Speed {
    Immediate = 0,
    Fast = 1,
    Normal = 2,
    Slow = 3,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum VelocityCurve {
    Light = 0,
    Normal = 1,
    Heavy = 2,
    Const = 3,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum ButtonBehaviour {
    Momentary = 0,
    Toggle = 1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Scene {
    /// 0..=15
    midi_channel: u8,
    pitch_bend_speed: Speed,
    /// 64 +/- 12 => -12..12
    transpose: u8,
    velocity_curve: VelocityCurve,
    /// 1..=127
    velocity_constant_value: u8,
    mod_enable: bool,
    /// 0..=127
    mod_cc: u8,
    mod_behaviour: ButtonBehaviour,
    /// 0..=127
    mod_off_value: u8,
    /// 0..=127
    mod_on_value: u8,
    mod_speed: Speed,
    sustain_enable: bool,
    /// 0..=127
    sustain_cc: u8,
    sustain_behaviour: ButtonBehaviour,
    /// 0..=127
    sustain_off_value: u8,
    /// 0..=127
    sustain_on_value: u8,
    sustain_speed: Speed,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            midi_channel: 0,
            pitch_bend_speed: Speed::Normal,
            transpose: 64,
            velocity_curve: VelocityCurve::Normal,
            velocity_constant_value: 100,
            mod_enable: true,
            mod_cc: 1, // mod wheel
            mod_behaviour: ButtonBehaviour::Momentary,
            mod_off_value: 0,
            mod_on_value: 127,
            mod_speed: Speed::Normal,
            sustain_enable: true,
            sustain_cc: 64, // sustain
            sustain_behaviour: ButtonBehaviour::Momentary,
            sustain_off_value: 0,
            sustain_on_value: 127,
            sustain_speed: Speed::Normal,
        }
    }
}

#[derive(Error, Debug)]
#[error("invalid scene value for {name}: {value}")]
struct InvalidSceneParam {
    name: &'static str,
    value: u8,
}

impl InvalidSceneParam {
    #[inline(always)]
    pub const fn new(name: &'static str, value: u8) -> Self {
        Self { name, value }
    }
}

impl Scene {
    /// Scene deserialization, see nanoKEY2_MIDIimp.txt "Scene Data Dump Format"
    pub fn from_midi_bytes(data: &[u8; 74]) -> Result<Self, InvalidSceneParam> {
        Ok(Self {
            midi_channel: data[1],
            pitch_bend_speed: Speed::try_from(data[3])
                .map_err(|_| InvalidSceneParam::new("pitch_bend_speed", data[3]))?,
            transpose: data[10],
            velocity_curve: VelocityCurve::try_from(data[11])
                .map_err(|_| InvalidSceneParam::new("velocity_curve", data[11]))?,
            velocity_constant_value: data[12],
            mod_enable: data[19] != 0,
            mod_cc: data[20],
            mod_behaviour: ButtonBehaviour::try_from(data[21])
                .map_err(|_| InvalidSceneParam::new("mod_behaviour", data[21]))?,
            mod_off_value: data[22],
            mod_on_value: data[23],
            mod_speed: Speed::try_from(data[25])
                .map_err(|_| InvalidSceneParam::new("pitch_bend_speed", data[25]))?,
            sustain_enable: data[28] != 0,
            sustain_cc: data[29],
            sustain_behaviour: ButtonBehaviour::try_from(data[30])
                .map_err(|_| InvalidSceneParam::new("mod_behaviour", data[30]))?,
            sustain_off_value: data[31],
            sustain_on_value: data[33],
            sustain_speed: Speed::try_from(data[34])
                .map_err(|_| InvalidSceneParam::new("pitch_bend_speed", data[34]))?,
        })
    }

    /// Scene serialization, see nanoKEY2_MIDIimp.txt "Scene Data Dump Format"
    pub fn to_midi_bytes(&self) -> [u8; 74] {
        let mut data = [0x7fu8; 74];

        data[0] = 0b0111_1010;
        data[1] = self.midi_channel & 0b1111;
        data[3] = self.pitch_bend_speed.into();

        data[8] = 0b0111_0001;
        data[10] = self.transpose & 0b0111_1111;
        data[11] = self.velocity_curve.into();
        data[12] = self.velocity_constant_value & 0b0111_1111;

        data[16] = 0b0000_0011;
        data[19] = self.mod_enable as u8;
        data[20] = self.mod_cc & 0b0111_1111;
        data[21] = self.mod_behaviour.into();
        data[22] = self.mod_off_value & 0b0111_1111;
        data[23] = self.mod_on_value & 0b0111_1111;

        data[24] = 0b0000_0110;
        data[25] = self.mod_speed.into();
        data[28] = self.sustain_enable as u8;
        data[29] = self.sustain_cc & 0b0111_1111;
        data[30] = self.sustain_behaviour.into();
        data[31] = self.sustain_off_value & 0b0111_1111;

        data[32] = 0b0111_1100;
        data[33] = self.sustain_on_value & 0b0111_1111;
        data[34] = self.sustain_speed.into();

        // there is 1 extra byte, so 2nd-last byte has first bit set
        data[72] = 0b0000_0001;

        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // const FULL_DATA: [u8; 83] = [
    //     64, 0, 1, 17, 1, 127, 75, 64, 122, 0, 127, 2, 127, 127, 127, 127, 113, 127, 66, 1, 100,
    //     127, 127, 127, 3, 127, 127, 1, 1, 0, 0, 127, 6, 2, 127, 127, 1, 64, 0, 0, 124, 127, 2,
    //     127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127,
    //     127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127,
    //     127, 127, 127, 1, 127, 247,
    // ];
    const SCENE_A_DATA: [u8; 74] = [
        122, 0, 127, 2, 127, 127, 127, 127, 113, 127, 66, 1, 100, 127, 127, 127, 3, 127, 127, 1, 1,
        0, 0, 127, 6, 2, 127, 127, 1, 64, 0, 0, 124, 127, 2, 127, 127, 127, 127, 127, 127, 127,
        127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127,
        127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 1, 127,
    ];
    const SCENE_A: Scene = Scene {
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
    };

    #[test]
    fn test_parse() {
        let scene = Scene::from_midi_bytes(&SCENE_A_DATA).unwrap();
        assert_eq!(scene, SCENE_A);
    }

    #[test]
    fn test_serialize() {
        assert_eq!(SCENE_A.to_midi_bytes(), SCENE_A_DATA);
    }
}
