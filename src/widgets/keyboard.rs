use iced::widget::Action;
use iced::widget::canvas::{self, Frame, Path, Program};
use iced::{Color, Padding, Point, Rectangle, Renderer, Size, Theme};

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

const WHITE_ZOOM_LEVELS: [u8; 5] = [20, 24, 28, 34, 40];

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

const COLOR_WHITE: Color = Color::from_rgb(0.85, 0.85, 0.85);
const COLOR_BLACK: Color = Color::from_rgb(0.2, 0.2, 0.2);
const COLOR_PRESSED: Color = Color::from_rgb(0.4, 0.7, 1.0);
const COLOR_BORDER: Color = Color::BLACK;

fn draw_piano(
    frame: &mut Frame,
    pressed_keys: &[bool; 128],
    center_note: u8,
    note_width: u8,
    bounds: Rectangle,
) {
    // fill whole frame black for a simple border
    frame.fill(
        &Path::rectangle(bounds.position(), bounds.size()),
        COLOR_BORDER,
    );

    let sizes = Sizes::from_white(note_width);

    let center_x = bounds.center_x();

    // find top-left x position of note 0
    let origin: f32 = match center_note % 12 {
        // center note is black
        1 => center_x + sizes.cd_offset - sizes.white * (center_note / 12 * 7 + 1) as f32,
        3 => center_x - sizes.cd_offset - sizes.white * (center_note / 12 * 7 + 2) as f32,
        6 => center_x + sizes.fa_offset - sizes.white * (center_note / 12 * 7 + 4) as f32,
        8 => center_x + 0.0000000000000 - sizes.white * (center_note / 12 * 7 + 5) as f32,
        10 => center_x - sizes.fa_offset - sizes.white * (center_note / 12 * 7 + 6) as f32,
        // center note is white
        0 => center_x - sizes.white / 2.0 - sizes.white * (center_note / 12 * 7 + 0) as f32,
        2 => center_x - sizes.white / 2.0 - sizes.white * (center_note / 12 * 7 + 1) as f32,
        4 => center_x - sizes.white / 2.0 - sizes.white * (center_note / 12 * 7 + 2) as f32,
        5 => center_x - sizes.white / 2.0 - sizes.white * (center_note / 12 * 7 + 3) as f32,
        7 => center_x - sizes.white / 2.0 - sizes.white * (center_note / 12 * 7 + 4) as f32,
        9 => center_x - sizes.white / 2.0 - sizes.white * (center_note / 12 * 7 + 5) as f32,
        11 => center_x - sizes.white / 2.0 - sizes.white * (center_note / 12 * 7 + 6) as f32,
        _ => unreachable!(),
    }
    .round();

    // loop visible white notes ("idx" does not count black notes, so only 75 white notes in total)
    const fn white_idx_to_note(idx: u8) -> u8 {
        ((idx / 7) * 12)
            + match idx % 7 {
                0 => 0,
                1 => 2,
                2 => 4,
                3 => 5,
                4 => 7,
                5 => 9,
                6 => 11,
                _ => unreachable!(),
            }
    }
    let white_idx_min = ((bounds.x - origin) / sizes.white).floor().clamp(0.0, 75.0) as u8;
    let white_idx_max = ((bounds.x - origin + bounds.width) / sizes.white)
        .ceil()
        .clamp(0.0, 75.0) as u8;
    for draw_idx in white_idx_min..white_idx_max {
        let draw_rect = Path::rectangle(
            Point::new(origin + sizes.white * draw_idx as f32, bounds.y),
            Size::new(sizes.white - 1.0, bounds.height - 1.0), // `- 1.0` for a border
        );
        let color = if pressed_keys[white_idx_to_note(draw_idx) as usize] {
            COLOR_PRESSED
        } else {
            COLOR_WHITE
        };
        frame.fill(&draw_rect, color);
    }

    // loop visible black notes ("idx" does not count white notes, so only 53 black notes in total)
    const fn black_idx_to_note(idx: u8) -> u8 {
        ((idx / 5) * 12)
            + match idx % 5 {
                0 => 1,
                1 => 3,
                2 => 6,
                3 => 8,
                4 => 10,
                _ => unreachable!(),
            }
    }
    let black_idx_min = white_idx_min / 7 * 5
        + match white_idx_min % 7 {
            0 => 0,
            1 => 0,
            2 => 1,
            3 => 2,
            4 => 2,
            5 => 3,
            6 => 4,
            _ => unreachable!(),
        };
    let black_idx_max = white_idx_max / 7 * 5
        + match white_idx_max % 7 {
            0 => 0,
            1 => 1,
            2 => 1,
            3 => 2,
            4 => 3,
            5 => 4,
            6 => 4,
            _ => unreachable!(),
        }
        .min(52);
    for draw_idx in black_idx_min..black_idx_max {
        let draw_x = origin
            + ((draw_idx / 5) as f32 * sizes.white * 7.0)
            + match draw_idx % 5 {
                0 => sizes.white * 1.0 - sizes.black / 2.0 - sizes.cd_offset,
                1 => sizes.white * 2.0 - sizes.black / 2.0 + sizes.cd_offset,
                2 => sizes.white * 4.0 - sizes.black / 2.0 - sizes.fa_offset,
                3 => sizes.white * 5.0 - sizes.black / 2.0 + 0.0,
                4 => sizes.white * 6.0 - sizes.black / 2.0 + sizes.fa_offset,
                _ => unreachable!(),
            };
        let draw_rect = Path::rectangle(
            Point::new(draw_x, bounds.y),
            Size::new(sizes.black, (bounds.height * 0.6).round()),
        );
        let color = if pressed_keys[dbg!(black_idx_to_note(draw_idx)) as usize] {
            COLOR_PRESSED
        } else {
            COLOR_BLACK
        };
        frame.fill(&draw_rect, color);
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

        let white_key_width = 20.0;
        let black_key_width = 14.0;
        let bottom_bar_height = 20.0;
        let keys_height = bounds.height - bottom_bar_height;

        let offset_x = (bounds.width / 2.0) - center_x(self.root_note, white_key_width);

        draw_piano(
            &mut frame,
            &self.pressed_keys,
            self.root_note,
            white_key_width.round() as u8,
            bounds.shrink(Padding::default().bottom(bottom_bar_height)),
        );

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
