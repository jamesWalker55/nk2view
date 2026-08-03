# nk2view

[![Demonstration](https://github.com/jamesWalker55/nk2view/raw/refs/heads/main/docs/nk2view_Ab3u8I0yRS.jpg)](https://github.com/jamesWalker55/nk2view/raw/refs/heads/main/docs/nk2view_Ab3u8I0yRS.mp4)

A utility tool to edit parameters on the Korg nanoKEY2 keyboard.

Only tested on **Windows 11**. This tool is basically useless on Windows 10 since Windows 10 doesn't have the new MIDI service which allows multi-client MIDI.

## Status

**Not complete.**

The Sustain + Modulation buttons are not implemented. Right now clicking on those button just crashes the program with a `todo!()` call.

Everything else is implemented and working, including:

- Clicking on keyboard to transpose
- Reconnecting to keyboard (Refresh button)
- Change keyboard MIDI channel
- Change velocity curve (light/normal/hard/constant)
- Zoom in/out piano keyboard view
- Persist current settings to keyboard memory (Save button)
