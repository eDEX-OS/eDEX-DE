#![allow(non_upper_case_globals)]

use xkbcommon::xkb::keysyms::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool,
}

pub fn keysym_to_bytes(keysym: u32, modifiers: Modifiers) -> Option<Vec<u8>> {
    let bytes = match keysym {
        KEY_Return | KEY_KP_Enter => b"\r".to_vec(),
        KEY_BackSpace => b"\x7f".to_vec(),
        KEY_Tab => {
            if modifiers.shift {
                b"\x1b[Z".to_vec()
            } else {
                b"\t".to_vec()
            }
        }
        KEY_Escape => b"\x1b".to_vec(),
        KEY_Up => b"\x1b[A".to_vec(),
        KEY_Down => b"\x1b[B".to_vec(),
        KEY_Right => b"\x1b[C".to_vec(),
        KEY_Left => b"\x1b[D".to_vec(),
        KEY_Home => b"\x1b[H".to_vec(),
        KEY_End => b"\x1b[F".to_vec(),
        KEY_Page_Up => b"\x1b[5~".to_vec(),
        KEY_Page_Down => b"\x1b[6~".to_vec(),
        KEY_Delete => b"\x1b[3~".to_vec(),
        KEY_Insert => b"\x1b[2~".to_vec(),
        KEY_F1 => b"\x1bOP".to_vec(),
        KEY_F2 => b"\x1bOQ".to_vec(),
        KEY_F3 => b"\x1bOR".to_vec(),
        KEY_F4 => b"\x1bOS".to_vec(),
        KEY_F5 => b"\x1b[15~".to_vec(),
        KEY_F6 => b"\x1b[17~".to_vec(),
        KEY_F7 => b"\x1b[18~".to_vec(),
        KEY_F8 => b"\x1b[19~".to_vec(),
        KEY_F9 => b"\x1b[20~".to_vec(),
        KEY_F10 => b"\x1b[21~".to_vec(),
        KEY_F11 => b"\x1b[23~".to_vec(),
        KEY_F12 => b"\x1b[24~".to_vec(),
        _ => return ctrl_bytes(keysym, modifiers).map(|bytes| apply_alt_prefix(bytes, modifiers)),
    };

    Some(apply_alt_prefix(bytes, modifiers))
}

pub fn key_event_to_bytes(keysym: u32, text: &str, modifiers: Modifiers) -> Option<Vec<u8>> {
    if let Some(bytes) = keysym_to_bytes(keysym, modifiers) {
        return Some(bytes);
    }

    if modifiers.ctrl {
        return ctrl_bytes(keysym, modifiers).map(|bytes| apply_alt_prefix(bytes, modifiers));
    }

    if text.is_empty() {
        return None;
    }

    let mut bytes = Vec::new();
    if modifiers.alt {
        bytes.push(0x1b);
    }
    bytes.extend_from_slice(text.as_bytes());
    Some(bytes)
}

fn ctrl_bytes(keysym: u32, modifiers: Modifiers) -> Option<Vec<u8>> {
    if !modifiers.ctrl {
        return None;
    }

    let byte = match keysym {
        KEY_at | KEY_2 => 0x00,
        KEY_bracketleft | KEY_3 => 0x1b,
        KEY_backslash | KEY_4 => 0x1c,
        KEY_bracketright | KEY_5 => 0x1d,
        KEY_asciicircum | KEY_6 => 0x1e,
        KEY_underscore | KEY_7 | KEY_slash => 0x1f,
        KEY_space => 0x00,
        _ => {
            let ch = char::from_u32(keysym)?;
            let upper = ch.to_ascii_uppercase();
            if upper.is_ascii_uppercase() {
                (upper as u8) & 0x1f
            } else {
                return None;
            }
        }
    };

    Some(vec![byte])
}

fn apply_alt_prefix(mut bytes: Vec<u8>, modifiers: Modifiers) -> Vec<u8> {
    if modifiers.alt {
        let mut prefixed = Vec::with_capacity(bytes.len() + 1);
        prefixed.push(0x1b);
        prefixed.append(&mut bytes);
        prefixed
    } else {
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::{key_event_to_bytes, Modifiers};
    use xkbcommon::xkb::keysyms::{KEY_Return, KEY_C};

    #[test]
    fn maps_enter() {
        assert_eq!(
            key_event_to_bytes(KEY_Return, "", Modifiers::default()),
            Some(b"\r".to_vec())
        );
    }

    #[test]
    fn maps_ctrl_c() {
        let modifiers = Modifiers {
            ctrl: true,
            ..Modifiers::default()
        };
        assert_eq!(key_event_to_bytes(KEY_C, "", modifiers), Some(vec![0x03]));
    }
}
