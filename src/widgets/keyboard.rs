use iced::widget::Action;
use iced::widget::canvas::{self, Frame, Path, Program, Text};
use iced::widget::text::LineHeight;
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

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

const COLOR_WHITE: Color = Color::from_rgb(0.85, 0.85, 0.85);
const COLOR_BLACK: Color = Color::from_rgb(0.2, 0.2, 0.2);
const COLOR_PRESSED: Color = Color::from_rgb(0.4, 0.7, 1.0);
const COLOR_BORDER: Color = Color::BLACK;
const COLOR_BAR_BG: Color = Color::WHITE;
const COLOR_BAR_HIGHLIGHT: Color = Color::from_rgb(1.0, 0.0, 0.0);
const COLOR_TEXT: Color = Color::BLACK;

const NUM_WHITE_KEYS: u8 = 75;
const NUM_BLACK_KEYS: u8 = 53;

const BOTTOM_BAR_HEIGHT: f32 = 4.0;

/// The range of notes that respond to clicks, centered on middle C.
/// `CLICKABLE_MIN_WHITE_IDX`/`MAX_WHITE_IDX` below are *derived* from these
/// two numbers, so the status-bar highlight can never drift out of sync
/// with what's actually clickable in `update()`.
const CLICKABLE_CENTER_NOTE: u8 = 60; // C4
const CLICKABLE_RANGE_SEMITONES: u8 = 12; // one octave each direction
const CLICKABLE_MIN_NOTE: u8 = CLICKABLE_CENTER_NOTE - CLICKABLE_RANGE_SEMITONES;
const CLICKABLE_MAX_NOTE: u8 = CLICKABLE_CENTER_NOTE + CLICKABLE_RANGE_SEMITONES;

// Evaluated at compile time (these are `const`, and `note_to_white_idx` is a
// `const fn`). If CLICKABLE_MIN_NOTE/MAX_NOTE above ever stop landing on
// white keys, this fails to *compile* instead of panicking the first time
// someone clicks near the edge of the range.
const CLICKABLE_MIN_WHITE_IDX: u8 = note_to_white_idx(CLICKABLE_MIN_NOTE);
const CLICKABLE_MAX_WHITE_IDX: u8 = note_to_white_idx(CLICKABLE_MAX_NOTE);

const fn is_clickable(note: u8) -> bool {
    CLICKABLE_MIN_NOTE <= note && note <= CLICKABLE_MAX_NOTE
}

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

/// Inverse of `white_idx_to_note`. Only valid for notes that are actually
/// white keys — panics otherwise (at compile time, if called from a const
/// context, as it is above).
const fn note_to_white_idx(note: u8) -> u8 {
    (note / 12) * 7
        + match note % 12 {
            0 => 0,
            2 => 1,
            4 => 2,
            5 => 3,
            7 => 4,
            9 => 5,
            11 => 6,
            _ => panic!("note_to_white_idx called with a black note"),
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
        8 => center_x - sizes.white * (center_note / 12 * 7 + 5) as f32,
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
        }
        .round();
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

/// Returns the note under `point` (coordinates relative to `rect_keys`),
/// if any. Checks black keys first since they're drawn on top of white keys.
fn hit_test_piano(
    point: Point,
    center_note: u8,
    note_width: u8,
    rect_keys: Rectangle,
) -> Option<u8> {
    let sizes = Sizes::from_white(note_width);
    let origin = compute_origin(rect_keys.center_x(), center_note, &sizes);

    visible_black_keys(&sizes, origin, &rect_keys)
        .chain(visible_white_keys(&sizes, origin, &rect_keys))
        .find(|key| key.rect.contains(point))
        .map(|key| key.note)
}

/// Splits the widget's full bounds into the piano-key area and the status
/// bar beneath it. Both `update` and `draw` need this same split, so it
/// lives in one place instead of being recomputed independently in each.
fn split_bounds(total_size: Size) -> (Rectangle, Rectangle) {
    let keys_height = (total_size.height - BOTTOM_BAR_HEIGHT).max(0.0);
    let rect_keys = Rectangle {
        x: 0.0,
        y: 0.0,
        width: total_size.width,
        height: keys_height,
    };
    let rect_bar = Rectangle {
        x: 0.0,
        y: keys_height,
        width: total_size.width,
        height: BOTTOM_BAR_HEIGHT,
    };
    (rect_keys, rect_bar)
}

fn draw_keys(
    frame: &mut Frame,
    pressed_keys: &[bool; 128],
    sizes: &Sizes,
    origin: f32,
    rect_keys: Rectangle,
) {
    for key in visible_white_keys(sizes, origin, &rect_keys) {
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
    for key in visible_black_keys(sizes, origin, &rect_keys) {
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
    // draw C1..C7 text
    let rect_labels_height = (rect_keys.height * 0.4).round().min(20.0);
    let rect_labels = Rectangle {
        x: rect_keys.x,
        y: rect_keys.y + rect_keys.height - rect_labels_height,
        width: rect_keys.width,
        height: rect_labels_height,
    };
    for i in 0..=10 {
        let rect = white_key_rect(i * 7, sizes, origin, &rect_labels);
        frame.fill_text(Text {
            content: format!("C{}", (i as i8) - 1),
            position: rect.center(),
            max_width: rect.width,
            color: COLOR_TEXT,
            size: 10.0.into(),
            line_height: LineHeight::default(),
            font: Default::default(),
            align_x: iced::widget::text::Alignment::Center,
            align_y: iced::alignment::Vertical::Center,
            shaping: iced::widget::text::Shaping::Basic,
        });
    }
}

fn draw_range_indicator(
    frame: &mut Frame,
    sizes: &Sizes,
    origin: f32,
    rect_keys: Rectangle,
    rect_bar: Rectangle,
    root_note: u8,
) {
    // bg of the indicator, spanning exactly the clickable range
    let bounding_rect = {
        let leftmost = white_key_rect(CLICKABLE_MIN_WHITE_IDX, sizes, origin, &rect_keys);
        let rightmost = white_key_rect(CLICKABLE_MAX_WHITE_IDX, sizes, origin, &rect_keys);
        Rectangle {
            x: leftmost.x,
            y: rect_bar.y,
            width: rightmost.x + rightmost.width - leftmost.x,
            height: rect_bar.height,
        }
    };
    frame.fill(
        &Path::rectangle(
            Point {
                x: bounding_rect.x + 1.0,
                y: bounding_rect.y,
            },
            Size {
                width: bounding_rect.width - 1.0,
                height: bounding_rect.height - 1.0,
            },
        ),
        COLOR_BAR_BG,
    );

    // the actual center indicator
    let width = match root_note % 12 {
        // black note
        1 | 3 | 6 | 8 | 10 => sizes.black,
        // white note
        _ => sizes.white,
    };
    let indicator_inner_rect = Rectangle {
        x: (rect_keys.center_x() - width / 2.0).round(),
        y: bounding_rect.y,
        width: width - 1.0,
        height: bounding_rect.height - 1.0,
    };
    let indicator_outer_rect = indicator_inner_rect.expand(1.0);
    frame.fill(
        &Path::rectangle(indicator_outer_rect.position(), indicator_outer_rect.size()),
        COLOR_BORDER,
    );
    frame.fill(
        &Path::rectangle(indicator_inner_rect.position(), indicator_inner_rect.size()),
        COLOR_BAR_HIGHLIGHT,
    );
}

pub struct KeyboardProgram<'a, Message> {
    pub note_width: u8,
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
            let (rect_keys, _) = split_bounds(bounds.size());
            let clicked_note = hit_test_piano(position, self.root_note, self.note_width, rect_keys);
            if let Some(clicked_note) = clicked_note
                && is_clickable(clicked_note)
            {
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
        raw_bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let sizes = Sizes::from_white(self.note_width);
        let mut frame = Frame::new(renderer, raw_bounds.size());

        // fill whole area for easy border
        frame.fill(
            &Path::rectangle(Point::ORIGIN, raw_bounds.size()),
            COLOR_BORDER,
        );

        let (rect_keys, rect_bar) = split_bounds(raw_bounds.size());
        let origin = compute_origin(rect_keys.center_x(), self.root_note, &sizes);

        draw_keys(&mut frame, self.pressed_keys, &sizes, origin, rect_keys);
        draw_range_indicator(
            &mut frame,
            &sizes,
            origin,
            rect_keys,
            rect_bar,
            self.root_note,
        );

        vec![frame.into_geometry()]
    }
}
