//! Tastetrykk og mus fra crossterm, oversatt til argumenter for `tmux send-keys`.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// Prefikset fra tmux-oppsettet, så tastene er de samme som i tmux.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prefiks {
    tegn: char,
    ctrl: bool,
    alt: bool,
}

impl Default for Prefiks {
    fn default() -> Prefiks {
        Prefiks { tegn: 'b', ctrl: true, alt: false }
    }
}

impl Prefiks {
    /// Fra navnet tmux bruker, som `C-b`, `M-a` eller `C-Space`.
    pub fn fra_tmux(navn: &str) -> Option<Prefiks> {
        let (mut ctrl, mut alt, mut rest) = (false, false, navn.trim());
        loop {
            if let Some(r) = rest.strip_prefix("C-") {
                (ctrl, rest) = (true, r);
            } else if let Some(r) = rest.strip_prefix("M-") {
                (alt, rest) = (true, r);
            } else {
                break;
            }
        }
        let tegn = match rest {
            "Space" => ' ',
            _ => {
                let mut tegn = rest.chars();
                let (Some(c), None) = (tegn.next(), tegn.next()) else { return None };
                c
            }
        };
        Some(Prefiks { tegn, ctrl, alt })
    }

    pub fn er(&self, k: &KeyEvent) -> bool {
        let KeyCode::Char(c) = k.code else { return false };
        let samme = if self.ctrl { c.eq_ignore_ascii_case(&self.tegn) } else { c == self.tegn };
        samme
            && k.modifiers.contains(KeyModifiers::CONTROL) == self.ctrl
            && k.modifiers.contains(KeyModifiers::ALT) == self.alt
    }

    pub fn tast(&self) -> KeyEvent {
        let mut m = KeyModifiers::NONE;
        m.set(KeyModifiers::CONTROL, self.ctrl);
        m.set(KeyModifiers::ALT, self.alt);
        KeyEvent::new(KeyCode::Char(self.tegn), m)
    }

    pub fn navn(&self) -> String {
        let tegn = if self.tegn == ' ' { "Space".to_string() } else { self.tegn.to_string() };
        format!("{}{}{tegn}", if self.ctrl { "C-" } else { "" }, if self.alt { "M-" } else { "" })
    }
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
        MouseEventKind::Moved => (3 + 32, 'M'),
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
    fn prefiks_fra_tmux() {
        let p = Prefiks::fra_tmux("C-a").unwrap();
        assert!(p.er(&KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL)));
        assert!(!p.er(&KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)));
        assert_eq!(p.navn(), "C-a");
        assert_eq!(Prefiks::fra_tmux("C-Space").map(|p| p.navn()).as_deref(), Some("C-Space"));
        assert_eq!(Prefiks::fra_tmux("`").map(|p| p.navn()).as_deref(), Some("`"));
        assert_eq!(Prefiks::fra_tmux("F12"), None);
        assert_eq!(send_keys(&Prefiks::default().tast()).as_deref(), Some("-H 02"));
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
        // \e[<35;1;1M
        let flytt = mus(&m(MouseEventKind::Moved, KeyModifiers::NONE), 0, 0);
        assert_eq!(flytt.as_deref(), Some("-H 1b 5b 3c 33 35 3b 31 3b 31 4d"));
    }
}
