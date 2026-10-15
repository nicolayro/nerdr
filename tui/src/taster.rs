//! Tastetrykk fra crossterm, oversatt til argumenter for `tmux send-keys`.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Ctrl-] bytter mellom lista og panen. Terminalen sender 0x1d for den, og
/// crossterm melder 0x1c–0x1f som Ctrl-4 til Ctrl-7.
pub fn er_bytt_fokus(k: &KeyEvent) -> bool {
    k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char(']' | '5'))
}

/// Tegn sendes som hex-bytes, så ingenting i dem kan tolkes av tmux sin
/// kommandoparser. Spesialtaster sendes med navn, så tmux koder dem slik panen
/// har bedt om, for eksempel piltaster i applikasjonsmodus.
pub fn send_keys(k: &KeyEvent) -> Option<String> {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let alt = k.modifiers.contains(KeyModifiers::ALT);
    let shift = k.modifiers.contains(KeyModifiers::SHIFT);

    let navn = match k.code {
        KeyCode::Char(c) => {
            let mut bytes = if alt { vec![0x1b] } else { Vec::new() };
            if ctrl {
                bytes.push(kontrolltegn(c)?);
            } else {
                bytes.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
            }
            return Some(format!("-H {}", hex(&bytes)));
        }
        KeyCode::BackTab => return Some("BTab".into()),
        KeyCode::F(n) => return Some(format!("{}F{n}", prefiks(ctrl, alt, shift))),
        KeyCode::Enter => "Enter",
        KeyCode::Tab => "Tab",
        KeyCode::Backspace => "BSpace",
        KeyCode::Esc => "Escape",
        KeyCode::Up => "Up",
        KeyCode::Down => "Down",
        KeyCode::Left => "Left",
        KeyCode::Right => "Right",
        KeyCode::Home => "Home",
        KeyCode::End => "End",
        KeyCode::PageUp => "PPage",
        KeyCode::PageDown => "NPage",
        KeyCode::Delete => "DC",
        KeyCode::Insert => "IC",
        _ => return None,
    };
    Some(format!("{}{navn}", prefiks(ctrl, alt, shift)))
}

/// Innlimt tekst, pakket i bracketed paste så Claude ikke sender inn ved
/// første linjeskift. Delt opp så ingen kommandolinje til tmux blir for lang.
pub fn lim_inn(tekst: &str) -> Vec<String> {
    let mut bytes = b"\x1b[200~".to_vec();
    bytes.extend(tekst.replace("\r\n", "\r").replace('\n', "\r").bytes());
    bytes.extend(b"\x1b[201~");
    bytes.chunks(256).map(|del| format!("-H {}", hex(del))).collect()
}

fn prefiks(ctrl: bool, alt: bool, shift: bool) -> String {
    [(ctrl, "C-"), (alt, "M-"), (shift, "S-")]
        .into_iter()
        .filter_map(|(på, p)| på.then_some(p))
        .collect()
}

fn kontrolltegn(c: char) -> Option<u8> {
    Some(match c.to_ascii_lowercase() {
        c @ 'a'..='z' => c as u8 - b'a' + 1,
        ' ' | '@' | '2' => 0x00,
        '[' | '3' => 0x1b,
        '\\' | '4' => 0x1c,
        ']' | '5' => 0x1d,
        '^' | '6' => 0x1e,
        '_' | '/' | '7' => 0x1f,
        '?' | '8' => 0x7f,
        _ => return None,
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tester {
    use super::*;

    fn tast(code: KeyCode, m: KeyModifiers) -> Option<String> {
        send_keys(&KeyEvent::new(code, m))
    }

    #[test]
    fn tegn_som_hex() {
        assert_eq!(tast(KeyCode::Char('æ'), KeyModifiers::NONE).as_deref(), Some("-H c3 a6"));
        assert_eq!(tast(KeyCode::Char('c'), KeyModifiers::CONTROL).as_deref(), Some("-H 03"));
        assert_eq!(tast(KeyCode::Char('b'), KeyModifiers::ALT).as_deref(), Some("-H 1b 62"));
    }

    #[test]
    fn spesialtaster_med_navn() {
        assert_eq!(tast(KeyCode::Up, KeyModifiers::NONE).as_deref(), Some("Up"));
        assert_eq!(tast(KeyCode::Enter, KeyModifiers::SHIFT).as_deref(), Some("S-Enter"));
        assert_eq!(tast(KeyCode::Left, KeyModifiers::ALT).as_deref(), Some("M-Left"));
    }

    #[test]
    fn innliming_pakkes_og_deles() {
        let deler = lim_inn("a\nb");
        assert_eq!(deler, vec!["-H 1b 5b 32 30 30 7e 61 0d 62 1b 5b 32 30 31 7e"]);
        assert_eq!(lim_inn(&"x".repeat(600)).len(), 3);
    }
}
