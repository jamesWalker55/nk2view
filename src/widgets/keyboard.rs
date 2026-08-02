use iced::widget::Action;
use iced::widget::canvas::{self, Frame, Path, Program};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

const WHITE_NOTES: [u8; 75] = [
    000, 002, 004, 005, 007, 009, 011, 012, 014, 016, 017, 019, 021, 023, 024, 026, 028, 029, 031,
    033, 035, 036, 038, 040, 041, 043, 045, 047, 048, 050, 052, 053, 055, 057, 059, 060, 062, 064,
    065, 067, 069, 071, 072, 074, 076, 077, 079, 081, 083, 084, 086, 088, 089, 091, 093, 095, 096,
    098, 100, 101, 103, 105, 107, 108, 110, 112, 113, 115, 117, 119, 120, 122, 124, 125, 127,
];
const BLACK_NOTES: [u8; 53] = [
    001, 003, 006, 008, 010, 013, 015, 018, 020, 022, 025, 027, 030, 032, 034, 037, 039, 042, 044,
    046, 049, 051, 054, 056, 058, 061, 063, 066, 068, 070, 073, 075, 078, 080, 082, 085, 087, 090,
    092, 094, 097, 099, 102, 104, 106, 109, 111, 114, 116, 118, 121, 123, 126,
];

fn visible_note_range(center_note: u8, note_width: u8, bounds: Rectangle) {
    let sizes = Sizes::from_white(note_width);

    let center_x = bounds.x + bounds.width / 2.0;

    // black notes are slightly off-center, find their X offset
    let black_x_offset = match center_note % 12 {
        1 => -sizes.cd_offset,
        3 => sizes.cd_offset,
        6 => -sizes.fa_offset,
        8 => 0.0,
        10 => sizes.fa_offset,
        _ => 0.0,
    };

    match WHITE_NOTES.binary_search(&center_note) {
        Ok(center_idx) => {
            // find top-left x position of note 0
            // we are centered on a white note, there are `i` additional notes to our left
            let origin: f32 =
                (center_x - sizes.white / 2.0 - sizes.white * center_idx as f32).round();

            // how many white notes are visible to the left/right of our centered note?
            let additional_white_notes_per_side =
                ((bounds.width / 2.0 - sizes.white / 2.0) / sizes.white).ceil() as usize;

            // loop visible white notes
            let white_min_idx = center_idx
                .checked_sub(additional_white_notes_per_side)
                .unwrap_or(0);
            let white_max_idx =
                (center_idx + additional_white_notes_per_side).min(WHITE_NOTES.len() - 1);
            for draw_idx in white_min_idx..=white_max_idx {
                let draw_note = WHITE_NOTES[draw_idx];
                let draw_rect = Path::rectangle(
                    Point::new(origin + sizes.white * draw_idx as f32, bounds.y),
                    Size::new(sizes.white, bounds.height),
                );
            }

            // loop visible black notes
            let black_min_note = WHITE_NOTES[white_min_idx]
                .checked_sub(1)
                .unwrap_or(1)
                .max(1);
            let black_max_note = (WHITE_NOTES[white_max_idx] + 1).max(126);
            for draw_note in black_min_note..=black_max_note {
                let draw_x = match draw_note % 12 {
                    1 => {
                        origin + (draw_note / 12) as f32 * sizes.white * 7.0 + sizes.white * 1.0
                            - sizes.black / 2.0
                            - sizes.cd_offset
                    }
                    3 => {
                        origin + (draw_note / 12) as f32 * sizes.white * 7.0 + sizes.white * 2.0
                            - sizes.black / 2.0
                            + sizes.cd_offset
                    }
                    6 => {
                        origin + (draw_note / 12) as f32 * sizes.white * 7.0 + sizes.white * 4.0
                            - sizes.black / 2.0
                            - sizes.fa_offset
                    }
                    8 => {
                        origin + (draw_note / 12) as f32 * sizes.white * 7.0 + sizes.white * 5.0
                            - sizes.black / 2.0
                            + 0.0
                    }
                    10 => {
                        origin + (draw_note / 12) as f32 * sizes.white * 7.0 + sizes.white * 6.0
                            - sizes.black / 2.0
                            + sizes.fa_offset
                    }
                    // skip non-black note
                    _ => continue,
                };
                let draw_rect = Path::rectangle(
                    Point::new(draw_x, bounds.y),
                    Size::new(sizes.black, bounds.height * 0.6),
                );
            }
        }
        Err(i) => {
            // find top-left x position of note 0
            // we are centered on a black note, there are `i` white notes to our left
            let origin: f32 = (center_x - black_x_offset - sizes.white * i as f32).round();
        }
    }

    // draw visible white notes

    // let extend_bounds_by = ((window_width / note_width).ceil() / 2.0).ceil() as usize + 1;
    // let (white_min, white_max) = match WHITE_NOTES.binary_search(&center_note) {
    //     Ok(i) => {
    //         // we are centered on a white note
    //         let white_min = i.checked_sub(extend_bounds_by).unwrap_or(0);
    //         let white_max = (i + extend_bounds_by).min(WHITE_NOTES.len() - 1);
    //         (white_min, white_max)
    //     },
    //     Err(i) => {
    //         // we are centered on a black note
    //         let white_min = i.checked_sub(extend_bounds_by + 1).unwrap_or(0);
    //         let white_max = (i + extend_bounds_by).min(WHITE_NOTES.len() - 1);
    //         (white_min, white_max)
    //     },
    // };
    // let visible_white_notes = window_width / note_width + 4.0; // extend visible range by 4 notes
    todo!()
}

// /// For simplicity, this renders extra notes on each side to ensure black notes are rendered correctly
// fn visible_note_range(center_note: u8, note_width: u8, window_width: f32) {
//     debug_assert!(center_note <= 127, "center note must be within 0..=127: {center_note}");
//     // debug_assert!(note_width > 0.0, "note width must be larger than 0.0: {note_width}");
//     debug_assert!(window_width > 0.0, "window width must be larger than 0.0: {window_width}");

//     let sizes = Sizes::from_white(note_width);
//     match center_note % 12 {

//     }

//     // find top-left x position of note 0

//     // let extend_bounds_by = ((window_width / note_width).ceil() / 2.0).ceil() as usize + 1;
//     // let (white_min, white_max) = match WHITE_NOTES.binary_search(&center_note) {
//     //     Ok(i) => {
//     //         // we are centered on a white note
//     //         let white_min = i.checked_sub(extend_bounds_by).unwrap_or(0);
//     //         let white_max = (i + extend_bounds_by).min(WHITE_NOTES.len() - 1);
//     //         (white_min, white_max)
//     //     },
//     //     Err(i) => {
//     //         // we are centered on a black note
//     //         let white_min = i.checked_sub(extend_bounds_by + 1).unwrap_or(0);
//     //         let white_max = (i + extend_bounds_by).min(WHITE_NOTES.len() - 1);
//     //         (white_min, white_max)
//     //     },
//     // };
//     // let visible_white_notes = window_width / note_width + 4.0; // extend visible range by 4 notes
//     todo!()
// }

const WHITE_ZOOM_LEVELS: [u8; 5] = [20, 24, 28, 34, 40];

#[repr(u8)]
enum Pitch {
    C = 0,
    CS = 1,
    D = 2,
    DS = 3,
    E = 4,
    F = 5,
    FS = 6,
    G = 7,
    GS = 8,
    A = 9,
    AS = 10,
    B = 11,
}
impl Pitch {
    const fn is_black(&self) -> bool {
        matches!(self, Self::CS | Self::DS | Self::FS | Self::GS | Self::AS)
    }
}

struct Note {
    octave: u8,
    pitch: Pitch,
}
impl Note {
    const fn parse(note: u8) -> Self {
        let octave = note / 12;
        let pitch: Pitch = unsafe { std::mem::transmute(note % 12) };
        Self { octave, pitch }
    }
}

/// Amounts are in pixels
#[derive(Debug)]
struct Sizes {
    white: f32,
    black: f32,
    cd_offset: f32,
    fa_offset: f32,
}

impl Sizes {
    /// Very roughly calculated from these values
    /// ```plain
    ///                 diva    reaper big  reaper small    gemini ai
    /// white           40      67          20              23.5
    /// black           24      41          13              13.7
    /// cd offset       6       7           2               2.25
    /// fa offset       6       10          3               3.37
    /// black height    153     245         34
    /// white height    235     409         57
    /// ```
    /// height ratio is around 0.6
    const fn from_white(white: u8) -> Self {
        let black = (white as f32 * 0.55 + 2.0).round();
        let cd_offset = (white as f32 * 0.10).round();
        let fa_offset = (white as f32 * 0.15).round();
        Self {
            white: white as f32,
            black,
            cd_offset,
            fa_offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        dbg!(const { Sizes::from_white(20) });
        dbg!(const { Sizes::from_white(24) });
        dbg!(const { Sizes::from_white(28) });
        dbg!(const { Sizes::from_white(34) });
        dbg!(const { Sizes::from_white(40) });
        assert!(false);
    }
}

// Returns the visual index of a white key (ignores black keys).
fn white_index(n: u8) -> f32 {
    let octave = n / 12;
    let note = n % 12;
    let offsets = [0, 0, 1, 1, 2, 3, 3, 4, 4, 5, 5, 6];
    (octave as f32 * 7.0) + offsets[note as usize] as f32
}

fn is_black(n: u8) -> bool {
    matches!(n % 12, 1 | 3 | 6 | 8 | 10)
}

// Calculates the mathematical center X coordinate of any MIDI note
fn center_x(n: u8, white_key_width: f32) -> f32 {
    if is_black(n) {
        // Black keys visually sit exactly on the boundary of the adjacent white keys
        (white_index(n) + 1.0) * white_key_width
    } else {
        white_index(n) * white_key_width + white_key_width / 2.0
    }
}

pub struct KeyboardProgram<'a, Message> {
    pub pressed_keys: &'a [bool; 128],
    pub root_note: u8,
    // Callback to emit messages generically when a key is clicked
    pub on_note_clicked: Box<dyn Fn(u8) -> Message + 'a>,
}

impl<'a, Message> Program<Message> for KeyboardProgram<'a, Message> {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<Action<Message>> {
        if let canvas::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) =
            event
            && let Some(position) = cursor.position_in(bounds)
        {
            let white_key_width = 26.0;
            let black_key_width = 14.0;
            let bottom_bar_height = 20.0;
            let keys_height = bounds.height - bottom_bar_height;
            let black_key_height = keys_height * 0.6;

            // How much we need to shift the keyboard to center the root note
            let offset_x = (bounds.width / 2.0) - center_x(self.root_note, white_key_width);

            // Adjust position relative to note 0
            let relative_x = position.x - offset_x;
            let y = position.y;

            if y < keys_height {
                // 1. Check black keys first (they are physically drawn on top)
                if y < black_key_height {
                    for n in 0..128 {
                        if is_black(n) {
                            let cx = center_x(n, white_key_width);
                            if relative_x >= cx - black_key_width / 2.0
                                && relative_x <= cx + black_key_width / 2.0
                            {
                                return Some(
                                    Action::publish((self.on_note_clicked)(n)).and_capture(),
                                );
                            }
                        }
                    }
                }

                // 2. Check white keys if no black key was clicked
                for n in 0..128 {
                    if !is_black(n) {
                        let cx = center_x(n, white_key_width);
                        if relative_x >= cx - white_key_width / 2.0
                            && relative_x <= cx + white_key_width / 2.0
                        {
                            return Some(Action::publish((self.on_note_clicked)(n)).and_capture());
                        }
                    }
                }
            }
        }
        None
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let white_key_width = 26.0;
        let black_key_width = 14.0;
        let bottom_bar_height = 20.0;
        let keys_height = bounds.height - bottom_bar_height;
        let black_key_height = keys_height * 0.6;

        let offset_x = (bounds.width / 2.0) - center_x(self.root_note, white_key_width);

        let unpressed_white = Color::from_rgb(0.85, 0.85, 0.85);
        let unpressed_black = Color::from_rgb(0.2, 0.2, 0.2);
        // Using a distinct blue for live pressed notes so it doesn't clash with the red root note
        let pressed_color = Color::from_rgb(0.4, 0.7, 1.0);

        // Draw a black background. This easily creates the 1px black outline between keys!
        frame.fill(&Path::rectangle(Point::ORIGIN, bounds.size()), Color::BLACK);

        // 1. Draw White Keys
        for n in 0..128 {
            if !is_black(n) {
                let cx = center_x(n, white_key_width) + offset_x;

                // Optimization: Don't draw keys that are outside the window bounds
                if cx + white_key_width / 2.0 < 0.0 || cx - white_key_width / 2.0 > bounds.width {
                    continue;
                }

                let color = if self.pressed_keys[n as usize] {
                    pressed_color
                } else {
                    unpressed_white
                };

                // Shrinking the width by 1.0 creates a natural black border from the background
                let path = Path::rectangle(
                    Point::new(cx - white_key_width / 2.0, 0.0),
                    Size::new(white_key_width - 1.0, keys_height),
                );
                frame.fill(&path, color);
            }
        }

        // 2. Draw Black Keys
        for n in 0..128 {
            if is_black(n) {
                let cx = center_x(n, white_key_width) + offset_x;
                if cx + black_key_width / 2.0 < 0.0 || cx - black_key_width / 2.0 > bounds.width {
                    continue;
                }

                let color = if self.pressed_keys[n as usize] {
                    pressed_color
                } else {
                    unpressed_black
                };

                let path = Path::rectangle(
                    Point::new(cx - black_key_width / 2.0, 0.0),
                    Size::new(black_key_width, black_key_height),
                );
                frame.fill(&path, color);
            }
        }

        // 3. Draw Bottom Indicator Bar
        // (The background of the bar is already black from the base fill)

        // Find the X boundaries of C4 +- 12 (root_note - 12 to root_note + 12)
        let min_note = self.root_note.saturating_sub(12);
        let max_note = self.root_note.saturating_add(12).min(127);

        let min_cx = center_x(min_note, white_key_width) + offset_x;
        let max_cx = center_x(max_note, white_key_width) + offset_x;

        let span_left = if is_black(min_note) {
            min_cx - black_key_width / 2.0
        } else {
            min_cx - white_key_width / 2.0
        };
        let span_right = if is_black(max_note) {
            max_cx + black_key_width / 2.0
        } else {
            max_cx + white_key_width / 2.0
        };

        // Draw the white span range
        let span_path = Path::rectangle(
            Point::new(span_left, keys_height + 4.0),
            Size::new(span_right - span_left, bottom_bar_height - 8.0),
        );
        frame.fill(&span_path, Color::WHITE);

        // Draw the red root note indicator right in the center
        let root_cx = center_x(self.root_note, white_key_width) + offset_x;
        let root_path = Path::rectangle(
            Point::new(root_cx - white_key_width / 2.0, keys_height + 4.0),
            Size::new(white_key_width, bottom_bar_height - 8.0),
        );
        frame.fill(&root_path, Color::from_rgb(0.9, 0.1, 0.1));

        vec![frame.into_geometry()]
    }
}
