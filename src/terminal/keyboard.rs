use super::{Modifiers, Terminal};
use alacritty_terminal::term::TermMode;

impl Terminal {
    pub fn key_input(
        &self,
        key: &str,
        typed: Option<&str>,
        mods: Modifiers,
        ctrl_h: bool,
    ) -> Option<Vec<u8>> {
        encode(key, typed, mods, *self.term.mode(), ctrl_h)
    }
}

fn modifier(mods: Modifiers) -> u8 {
    1 + mods.shift as u8 + 2 * mods.alt as u8 + 4 * mods.control as u8
}

fn final_key(code: char, m: u8, application: bool) -> Vec<u8> {
    if m != 1 {
        format!("\x1b[1;{m}{code}").into_bytes()
    } else {
        vec![0x1b, if application { b'O' } else { b'[' }, code as u8]
    }
}

fn tilde_key(code: u8, m: u8) -> Vec<u8> {
    if m == 1 {
        format!("\x1b[{code}~").into_bytes()
    } else {
        format!("\x1b[{code};{m}~").into_bytes()
    }
}

fn modified_char(code: char, m: u8) -> Vec<u8> {
    format!("\x1b[27;{m};{}~", code as u32).into_bytes()
}

fn function_key(number: usize, mut mods: Modifiers) -> Option<Vec<u8>> {
    let index = number.checked_sub(1)?;
    match index / 12 {
        0 => {}
        1 => mods.shift = true,
        2 => mods.control = true,
        _ => return None,
    }
    let m = modifier(mods);
    Some(match index % 12 {
        n @ 0..=3 => final_key((b'P' + n as u8) as char, m, true),
        n => tilde_key([15, 17, 18, 19, 20, 21, 23, 24][n - 4], m),
    })
}

fn control_byte(key: char) -> Option<u8> {
    const RU: &str = "йцукенгшщзхъфывапролджэячсмитьбю";
    const EN: &str = "qwertyuiop[]asdfghjkl;'zxcvbnm,.";
    let lower = key.to_lowercase().next()?;
    let ascii = RU
        .chars()
        .position(|c| c == lower)
        .and_then(|n| EN.as_bytes().get(n))
        .copied();
    let c = ascii.map(char::from).unwrap_or(lower);
    Some(match c {
        'a'..='z' => c as u8 - b'a' + 1,
        ' ' | '@' | '2' => 0,
        '[' | '3' => 0x1b,
        '\\' | '4' => 0x1c,
        ']' | '5' => 0x1d,
        '^' | '6' => 0x1e,
        '_' | '-' | '/' | '7' => 0x1f,
        '?' | '8' => 0x7f,
        _ => return None,
    })
}

fn keypad(key: &str) -> Option<(char, char)> {
    let code = match key {
        "0" => ('0', 'p'),
        "1" => ('1', 'q'),
        "2" => ('2', 'r'),
        "3" => ('3', 's'),
        "4" => ('4', 't'),
        "5" => ('5', 'u'),
        "6" => ('6', 'v'),
        "7" => ('7', 'w'),
        "8" => ('8', 'x'),
        "9" => ('9', 'y'),
        "multiply" => ('*', 'j'),
        "add" => ('+', 'k'),
        "comma" => (',', 'l'),
        "subtract" => ('-', 'm'),
        "dot" => ('.', 'n'),
        "divide" => ('/', 'o'),
        "enter" => ('\r', 'M'),
        "equals" => ('=', 'X'),
        _ => return None,
    };
    Some(code)
}

fn encode(
    key: &str,
    typed: Option<&str>,
    mods: Modifiers,
    mode: TermMode,
    ctrl_h: bool,
) -> Option<Vec<u8>> {
    let m = modifier(mods);
    if let Some(number) = key.strip_prefix('f').and_then(|s| s.parse::<usize>().ok()) {
        return function_key(number, mods);
    }
    if let Some((normal, application)) = key.strip_prefix("numpad").and_then(keypad) {
        if mode.contains(TermMode::APP_KEYPAD) {
            return Some(final_key(application, m, true));
        }
        if normal == '\r' {
            return encode("enter", None, mods, mode, ctrl_h);
        }
        let mut buffer = [0; 4];
        let normal = normal.encode_utf8(&mut buffer);
        return encode(normal, Some(normal), mods, mode, ctrl_h);
    }
    let cursor = match key {
        "up" => Some('A'),
        "down" => Some('B'),
        "right" => Some('C'),
        "left" => Some('D'),
        "home" => Some('H'),
        "end" => Some('F'),
        _ => None,
    };
    if let Some(code) = cursor {
        return Some(final_key(code, m, mode.contains(TermMode::APP_CURSOR)));
    }
    let tilde = match key {
        "insert" => Some(2),
        "delete" => Some(3),
        "pageup" => Some(5),
        "pagedown" => Some(6),
        _ => None,
    };
    if let Some(code) = tilde {
        return Some(tilde_key(code, m));
    }

    let mut chars = key.chars();
    let single = chars.next().filter(|_| chars.next().is_none());
    let mut bytes = match key {
        "enter" if mods.control || mods.shift => return Some(modified_char('\r', m)),
        "enter" => vec![b'\r'],
        "backspace" => vec![if ctrl_h || mods.control { 8 } else { 0x7f }],
        "tab" if mods.control => return Some(modified_char('\t', m)),
        "tab" if mods.shift => b"\x1b[Z".to_vec(),
        "tab" => vec![b'\t'],
        "escape" => vec![0x1b],
        "space" if mods.control => vec![0],
        "space" => vec![b' '],
        _ if mods.control => {
            let c = single?;
            match control_byte(c) {
                Some(byte) => vec![byte],
                None if !c.is_control() => return Some(modified_char(c, m)),
                None => return None,
            }
        }
        _ => match typed.filter(|t| !t.is_empty()) {
            Some(text) => text.as_bytes().to_vec(),
            None if mods.alt => single?.encode_utf8(&mut [0; 4]).as_bytes().to_vec(),
            None => return None,
        },
    };
    if mods.alt {
        bytes.insert(0, 0x1b);
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_keys_send_bytes_without_typed_text() {
        let t = Terminal::new(80, 24);
        let ctrl = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        for (key, byte) in [
            ("c", 3),
            ("с", 3),
            ("В", 4),
            ("л", 11),
            ("х", 27),
            ("v", 22),
            ("space", 0),
            ("2", 0),
            ("[", 27),
            ("\\", 28),
            ("]", 29),
            ("6", 30),
            ("-", 31),
            ("8", 127),
        ] {
            assert_eq!(
                t.key_input(key, None, ctrl, false).unwrap(),
                [byte],
                "{key}"
            );
        }
        assert_eq!(
            t.key_input("tab", None, ctrl, false).unwrap(),
            b"\x1b[27;5;9~"
        );
        assert_eq!(
            t.key_input(",", None, ctrl, false).unwrap(),
            b"\x1b[27;5;44~"
        );
    }

    #[test]
    fn function_and_editing_keys_match_xterm() {
        let t = Terminal::new(80, 24);
        let none = Modifiers::default();
        let expected: &[&[u8]] = &[
            b"\x1bOP",
            b"\x1bOQ",
            b"\x1bOR",
            b"\x1bOS",
            b"\x1b[15~",
            b"\x1b[17~",
            b"\x1b[18~",
            b"\x1b[19~",
            b"\x1b[20~",
            b"\x1b[21~",
            b"\x1b[23~",
            b"\x1b[24~",
        ];
        for (i, bytes) in expected.iter().enumerate() {
            assert_eq!(
                t.key_input(&format!("f{}", i + 1), None, none, false)
                    .unwrap(),
                *bytes
            );
        }
        let ctrl_alt = Modifiers {
            control: true,
            alt: true,
            ..none
        };
        assert_eq!(t.key_input("f13", None, none, false).unwrap(), b"\x1b[1;2P");
        assert_eq!(
            t.key_input("insert", None, none, false).unwrap(),
            b"\x1b[2~"
        );
        assert_eq!(
            t.key_input("delete", None, ctrl_alt, false).unwrap(),
            b"\x1b[3;7~"
        );
        assert_eq!(
            t.key_input("f12", None, ctrl_alt, false).unwrap(),
            b"\x1b[24;7~"
        );
        assert_eq!(
            t.key_input("pageup", None, ctrl_alt, false).unwrap(),
            b"\x1b[5;7~"
        );
        assert_eq!(
            t.key_input("left", None, ctrl_alt, false).unwrap(),
            b"\x1b[1;7D"
        );
    }

    #[test]
    fn cursor_and_keypad_follow_program_modes() {
        let mut t = Terminal::new(80, 24);
        let mods = Modifiers::default();
        assert_eq!(t.key_input("up", None, mods, false).unwrap(), b"\x1b[A");
        assert_eq!(t.key_input("home", None, mods, false).unwrap(), b"\x1b[H");
        assert_eq!(t.key_input("numpad7", None, mods, false).unwrap(), b"7");
        t.feed(b"\x1b[?1h\x1b=");
        assert_eq!(t.key_input("up", None, mods, false).unwrap(), b"\x1bOA");
        assert_eq!(t.key_input("home", None, mods, false).unwrap(), b"\x1bOH");
        assert_eq!(
            t.key_input("numpad7", None, mods, false).unwrap(),
            b"\x1bOw"
        );
        assert_eq!(
            t.key_input("numpadenter", None, mods, false).unwrap(),
            b"\x1bOM"
        );
    }

    #[test]
    fn text_and_backspace_keep_their_encoding() {
        let t = Terminal::new(80, 24);
        let none = Modifiers::default();
        let alt = Modifiers { alt: true, ..none };
        let shift = Modifiers {
            shift: true,
            ..none
        };
        assert_eq!(t.key_input("enter", None, none, false).unwrap(), b"\r");
        assert_eq!(t.key_input("tab", None, shift, false).unwrap(), b"\x1b[Z");
        assert_eq!(t.key_input("b", None, alt, false).unwrap(), b"\x1bb");
        assert_eq!(
            t.key_input("x", Some("ж"), none, false).unwrap(),
            "ж".as_bytes()
        );
        assert_eq!(t.key_input("backspace", None, none, false).unwrap(), [127]);
        assert_eq!(
            t.key_input("backspace", None, alt, true).unwrap(),
            b"\x1b\x08"
        );
        assert_eq!(t.key_input("shift", None, none, false), None);
    }
}
