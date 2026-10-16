//! Tastetrykk og mus fra crossterm, oversatt til argumenter for `tmux send-keys`.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// Ctrl-Q bytter mellom lista og panen. Claude Code bruker den ikke, og den
/// ligger lett til også på nordiske tastatur, der `]` krever Option.
pub fn er_bytt_fokus(k: &KeyEvent) -> bool {
    k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('q')
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

/// Musehendelsen i SGR-koding (modus 1006), med `x` og `y` fra 0 i panen.
/// tmux kan ikke selv videresende mus fra en control mode-klient, så
/// sekvensen sendes som bytes.
pub fn mus(m: &MouseEvent, x: u16, y: u16) -> Option<String> {
    let (knapp, slutt) = match m.kind {
        MouseEventKind::Down(b) => (knappkode(b), 'M'),
        MouseEventKind::Up(b) => (knappkode(b), 'm'),
        MouseEventKind::Drag(b) => (knappkode(b) + 32, 'M'),
        MouseEventKind::ScrollUp => (64, 'M'),
        MouseEventKind::ScrollDown => (65, 'M'),
        _ => return None,
    };
    let tillegg: u16 = [(KeyModifiers::SHIFT, 4), (KeyModifiers::ALT, 8), (KeyModifiers::CONTROL, 16)]
        .into_iter()
        .filter_map(|(flagg, verdi)| m.modifiers.contains(flagg).then_some(verdi))
        .sum();
    let sekvens = format!("\x1b[<{};{};{}{slutt}", knapp + tillegg, x + 1, y + 1);
    Some(format!("-H {}", hex(sekvens.as_bytes())))
}

fn knappkode(b: MouseButton) -> u16 {
    match b {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    }
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
    fn mus_som_sgr() {
        let m = |kind, modifiers| MouseEvent { kind, column: 0, row: 0, modifiers };
        // \e[<64;3;1M
        let opp = mus(&m(MouseEventKind::ScrollUp, KeyModifiers::NONE), 2, 0);
        assert_eq!(opp.as_deref(), Some("-H 1b 5b 3c 36 34 3b 33 3b 31 4d"));
        // \e[<16;1;2m
        let slipp = mus(&m(MouseEventKind::Up(MouseButton::Left), KeyModifiers::CONTROL), 0, 1);
        assert_eq!(slipp.as_deref(), Some("-H 1b 5b 3c 31 36 3b 31 3b 32 6d"));
        assert_eq!(mus(&m(MouseEventKind::Moved, KeyModifiers::NONE), 0, 0), None);
    }
}
