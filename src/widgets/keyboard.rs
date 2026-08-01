use iced::widget::Action;
use iced::widget::canvas::{self, Frame, Path, Program};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

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
                            return Some(
                                Action::publish((self.on_note_clicked)(n)).and_capture(),
                            );
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
