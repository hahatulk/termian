use super::{Modifiers, Terminal};
use alacritty_terminal::term::TermMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left = 0,
    Middle = 1,
    Right = 2,
}

impl Button {
    fn mask(self) -> u8 {
        1 << self as u8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Press(Button),
    Release(Button),
    Move(Option<Button>),
    Scroll { columns: i32, rows: i32 },
}

#[derive(Default)]
pub(super) struct State {
    pressed: u8,
    pub(super) cell: Option<(usize, usize)>,
}

impl Terminal {
    pub fn mouse_tracking(&self) -> bool {
        self.term.mode().intersects(TermMode::MOUSE_MODE) && self.term.grid().display_offset() == 0
    }

    pub fn mouse_pressed(&self, button: Button) -> bool {
        self.mouse.pressed & button.mask() != 0
    }

    pub fn mouse_input(
        &mut self,
        event: Event,
        cell: (usize, usize),
        mods: Modifiers,
    ) -> Option<Vec<u8>> {
        let mode = *self.term.mode();
        if let Event::Release(button) = event {
            if !self.mouse_pressed(button) {
                return None;
            }
            self.mouse.pressed &= !button.mask();
            if !mode.intersects(TermMode::MOUSE_MODE) {
                return None;
            }
        } else if !self.mouse_tracking() || mods.shift {
            return None;
        }

        let (columns, rows) = self.size();
        if cell.0 >= rows || cell.1 >= columns {
            return None;
        }
        let bits = (mods.shift as u8) * 4 + (mods.alt as u8) * 8 + (mods.control as u8) * 16;
        let (code, release) = match event {
            Event::Press(button) => (button as u8, false),
            Event::Release(button) => (button as u8, true),
            Event::Move(button) => {
                let code = match button {
                    Some(button)
                        if self.mouse_pressed(button)
                            && mode.intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION) =>
                    {
                        button as u8
                    }
                    None if mode.contains(TermMode::MOUSE_MOTION) => 3,
                    _ => return None,
                };
                if self.mouse.cell == Some(cell) {
                    return None;
                }
                (32 + code, false)
            }
            Event::Scroll { columns, rows } => {
                let mut bytes = Vec::new();
                for (delta, up, down) in [(rows, 64, 65), (columns, 66, 67)] {
                    if delta != 0 {
                        let report = encode(
                            mode,
                            (if delta > 0 { up } else { down }) + bits,
                            cell,
                            false,
                        )?;
                        for _ in 0..delta.unsigned_abs().min(64) {
                            bytes.extend_from_slice(&report);
                        }
                    }
                }
                if bytes.is_empty() {
                    return None;
                }
                self.mouse.cell = Some(cell);
                return Some(bytes);
            }
        };
        let bytes = encode(mode, code + bits, cell, release)?;
        if let Event::Press(button) = event {
            self.mouse.pressed |= button.mask();
        }
        self.mouse.cell = Some(cell);
        Some(bytes)
    }
}

fn encode(
    mode: TermMode,
    code: u8,
    (row, column): (usize, usize),
    release: bool,
) -> Option<Vec<u8>> {
    let (x, y) = (column + 1, row + 1);
    if mode.contains(TermMode::SGR_MOUSE) {
        let suffix = if release { 'm' } else { 'M' };
        return Some(format!("\x1b[<{code};{x};{y}{suffix}").into_bytes());
    }
    let code = if release { (code & !3) | 3 } else { code };
    let mut bytes = vec![0x1b, b'[', b'M', code + 32];
    if mode.contains(TermMode::UTF8_MOUSE) {
        for coordinate in [x, y] {
            if coordinate > 2015 {
                return None;
            }
            bytes.extend_from_slice(
                char::from_u32(coordinate as u32 + 32)?
                    .encode_utf8(&mut [0; 4])
                    .as_bytes(),
            );
        }
    } else {
        if x > 223 || y > 223 {
            return None;
        }
        bytes.extend_from_slice(&[x as u8 + 32, y as u8 + 32]);
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Button::*;
    use Event::*;

    fn input(terminal: &mut Terminal, event: Event, row: usize, column: usize) -> Option<Vec<u8>> {
        terminal.mouse_input(event, (row, column), Modifiers::default())
    }

    #[test]
    fn sgr_click_pairs_press_and_release_at_the_requested_cell() {
        let mut t = Terminal::new(80, 24);
        assert_eq!(input(&mut t, Press(Left), 7, 16), None);
        t.feed(b"\x1b[?1000;1006h");
        assert_eq!(input(&mut t, Press(Left), 7, 16).unwrap(), b"\x1b[<0;17;8M");
        assert_eq!(input(&mut t, Move(Some(Left)), 7, 17), None);
        assert_eq!(
            input(&mut t, Release(Left), 7, 16).unwrap(),
            b"\x1b[<0;17;8m"
        );
        assert_eq!(input(&mut t, Release(Left), 7, 16), None);
        assert_eq!(
            input(&mut t, Press(Middle), 9, 12).unwrap(),
            b"\x1b[<1;13;10M"
        );
        assert_eq!(
            input(&mut t, Press(Right), 9, 12).unwrap(),
            b"\x1b[<2;13;10M"
        );
        t.feed(b"\x1b[?1000l");
        assert!(!t.mouse_pressed(Middle));
        assert!(!t.mouse_tracking());
    }

    #[test]
    fn drag_and_hover_follow_the_requested_tracking_mode() {
        let mut t = Terminal::new(80, 24);
        t.feed(b"\x1b[?1002;1006h");
        assert_eq!(input(&mut t, Move(None), 0, 1), None);
        assert_eq!(input(&mut t, Move(Some(Left)), 0, 1), None);
        input(&mut t, Press(Left), 0, 1).unwrap();
        assert_eq!(input(&mut t, Move(Some(Left)), 0, 1), None);
        assert_eq!(
            input(&mut t, Move(Some(Left)), 0, 2).unwrap(),
            b"\x1b[<32;3;1M"
        );
        assert_eq!(input(&mut t, Move(Some(Left)), 0, 2), None);
        input(&mut t, Release(Left), 0, 2).unwrap();
        t.feed(b"\x1b[?1003h");
        assert_eq!(input(&mut t, Move(None), 0, 3).unwrap(), b"\x1b[<35;4;1M");
    }

    #[test]
    fn shift_keeps_input_local_and_existing_drags_can_end() {
        let mut t = Terminal::new(80, 24);
        t.feed(b"\x1b[?1000;1006h");
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        assert_eq!(t.mouse_input(Press(Left), (1, 1), shift), None);
        assert_eq!(
            t.mouse_input(
                Scroll {
                    columns: 0,
                    rows: 1
                },
                (1, 1),
                shift
            ),
            None
        );
        input(&mut t, Press(Left), 1, 1).unwrap();
        assert_eq!(
            t.mouse_input(Release(Left), (1, 1), shift).unwrap(),
            b"\x1b[<4;2;2m"
        );
    }

    #[test]
    fn wheel_reports_direction_and_modifiers() {
        let mut t = Terminal::new(80, 24);
        t.feed(b"\x1b[?1000;1006h");
        let event = Scroll {
            columns: -1,
            rows: 2,
        };
        let mods = Modifiers {
            alt: true,
            control: true,
            shift: false,
        };
        assert_eq!(
            t.mouse_input(event, (0, 0), mods).unwrap(),
            b"\x1b[<88;1;1M\x1b[<88;1;1M\x1b[<91;1;1M"
        );
        assert_eq!(
            input(
                &mut t,
                Scroll {
                    columns: 1,
                    rows: -1
                },
                0,
                0
            )
            .unwrap(),
            b"\x1b[<65;1;1M\x1b[<66;1;1M"
        );
    }

    #[test]
    fn legacy_and_utf8_reports_use_their_coordinate_limits() {
        let mut t = Terminal::new(2040, 8);
        t.feed(b"\x1b[?1000h");
        assert_eq!(
            input(&mut t, Press(Right), 0, 222).unwrap(),
            b"\x1b[M\x22\xff\x21"
        );
        assert_eq!(
            input(&mut t, Release(Right), 0, 222).unwrap(),
            b"\x1b[M\x23\xff\x21"
        );
        assert_eq!(input(&mut t, Press(Left), 0, 223), None);
        t.feed(b"\x1b[?1005h");
        assert_eq!(
            input(&mut t, Press(Left), 0, 2014).unwrap(),
            b"\x1b[M \xdf\xbf!"
        );
        assert_eq!(input(&mut t, Press(Left), 0, 2015), None);
    }

    #[test]
    fn scrollback_cells_are_not_sent_as_application_coordinates() {
        let mut t = Terminal::new(20, 3);
        t.feed(b"a\r\nb\r\nc\r\nd\r\n\x1b[?1000;1006h");
        t.scroll(1);
        assert_eq!(input(&mut t, Press(Left), 0, 0), None);
        t.scroll(-1);
        assert_eq!(input(&mut t, Press(Left), 0, 0).unwrap(), b"\x1b[<0;1;1M");
    }
}
