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

/// Amounts are in logical pixels
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

const NUM_WHITE_KEYS: u8 = 75;
const NUM_BLACK_KEYS: u8 = 53;

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

/// x position of the top-left corner of white key idx 0, such that
/// `center_note` is centered at `center_x`.
fn compute_origin(center_x: f32, center_note: u8, sizes: &Sizes) -> f32 {
    match center_note % 12 {
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
    .round()
}

fn white_key_rect(idx: u8, sizes: &Sizes, origin: f32, bounds: &Rectangle) -> Rectangle {
    Rectangle::new(
        Point::new(origin + sizes.white * idx as f32, bounds.y),
        Size::new(sizes.white - 1.0, bounds.height - 1.0), // `- 1.0` for a border
    )
}

fn black_key_rect(idx: u8, sizes: &Sizes, origin: f32, bounds: &Rectangle) -> Rectangle {
    let left_x = origin
        + ((idx / 5) as f32 * sizes.white * 7.0)
        + match idx % 5 {
            0 => sizes.white * 1.0 - sizes.black / 2.0 - sizes.cd_offset,
            1 => sizes.white * 2.0 - sizes.black / 2.0 + sizes.cd_offset,
            2 => sizes.white * 4.0 - sizes.black / 2.0 - sizes.fa_offset,
            3 => sizes.white * 5.0 - sizes.black / 2.0,
            4 => sizes.white * 6.0 - sizes.black / 2.0 + sizes.fa_offset,
            _ => unreachable!(),
        };
    Rectangle::new(
        Point::new(left_x, bounds.y),
        Size::new(sizes.black, (bounds.height * 0.6).round()),
    )
}

/// Geometry of a single key, in the same coordinate space as `bounds`.
struct KeyGeometry {
    note: u8,
    rect: Rectangle,
}

/// Every black key visible within `bounds`. No index-range math — just
/// generate all 53 and keep the ones that overlap.
fn visible_black_keys<'a>(
    sizes: &'a Sizes,
    origin: f32,
    bounds: &'a Rectangle,
) -> impl Iterator<Item = KeyGeometry> + 'a {
    (0..NUM_BLACK_KEYS).filter_map(move |idx| {
        let rect = black_key_rect(idx, sizes, origin, bounds);
        rect.intersects(bounds).then(|| KeyGeometry {
            note: black_idx_to_note(idx),
            rect,
        })
    })
}

fn visible_white_keys<'a>(
    sizes: &'a Sizes,
    origin: f32,
    bounds: &'a Rectangle,
) -> impl Iterator<Item = KeyGeometry> + 'a {
    (0..NUM_WHITE_KEYS).filter_map(move |idx| {
        let rect = white_key_rect(idx, sizes, origin, bounds);
        rect.intersects(bounds).then(|| KeyGeometry {
            note: white_idx_to_note(idx),
            rect,
        })
    })
}

fn draw_piano(
    frame: &mut Frame,
    pressed_keys: &[bool; 128],
    center_note: u8,
    note_width: u8,
    bounds: Rectangle,
) {
    frame.fill(
        &Path::rectangle(bounds.position(), bounds.size()),
        COLOR_BORDER,
    );

    let sizes = Sizes::from_white(note_width);
    let origin = compute_origin(bounds.center_x(), center_note, &sizes);

    for key in visible_white_keys(&sizes, origin, &bounds) {
        let color = if pressed_keys[key.note as usize] {
            COLOR_PRESSED
        } else {
            COLOR_WHITE
        };
        frame.fill(
            &Path::rectangle(key.rect.position(), key.rect.size()),
            color,
        );
    }

    for key in visible_black_keys(&sizes, origin, &bounds) {
        let color = if pressed_keys[key.note as usize] {
            COLOR_PRESSED
        } else {
            COLOR_BLACK
        };
        frame.fill(
            &Path::rectangle(key.rect.position(), key.rect.size()),
            color,
        );
    }
}

/// Returns the note under `point` (canvas-local coordinates), if any.
/// Checks black keys first since they're drawn on top of white keys —
/// same visible set, same rects, as `draw_piano` used to paint them.
fn hit_test_piano(point: Point, center_note: u8, note_width: u8, bounds: Rectangle) -> Option<u8> {
    let sizes = Sizes::from_white(note_width);
    let origin = compute_origin(bounds.center_x(), center_note, &sizes);

    visible_black_keys(&sizes, origin, &bounds)
        .chain(visible_white_keys(&sizes, origin, &bounds))
        .find(|key| key.rect.contains(point))
        .map(|key| key.note)
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
    pub note_width: u8,
    pub pressed_keys: &'a [bool; 128],
    pub root_note: u8,
    // Callback to emit messages generically when a key is clicked
    pub on_note_clicked: Box<dyn Fn(u8) -> Message + 'a>,
}

const BOTTOM_BAR_HEIGHT: f32 = 0.0;

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
            let clicked_note = hit_test_piano(
                position,
                self.root_note,
                self.note_width,
                Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: bounds.width,
                    height: bounds.height - BOTTOM_BAR_HEIGHT,
                },
            );
            if let Some(clicked_note) = clicked_note
                && ((60 - 12) <= clicked_note && clicked_note <= (60 + 12))
            {
                // only allow clicking between 1 octave of C4 (60)
                return Some(Action::publish((self.on_note_clicked)(clicked_note)).and_capture());
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

        draw_piano(
            &mut frame,
            &self.pressed_keys,
            self.root_note,
            self.note_width,
            // a frame uses local coordinates, not global coordinates
            Rectangle {
                x: 0.0,
                y: 0.0,
                width: bounds.width,
                height: bounds.height - BOTTOM_BAR_HEIGHT,
            },
        );

        vec![frame.into_geometry()]
    }
}
