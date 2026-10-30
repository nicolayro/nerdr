//! Prefikstabellen fra tmux, med bindingene i oppsettet ditt, brukt i nerdr.
//! Kommandoer nerdr har en tast for, gjøres som med den tasten, så visningen
//! følger med. Kommandoer som ikke trenger et skjermbilde fra tmux, sendes til
//! tmux. Resten gjør ingenting her, og da er tasten som i nerdr. Standarden i
//! tmux blir dermed det samme som nerdr har fra før.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use nerdr_kjerne::bindinger::{Binding, kommandoer};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Det bare nerdr har. Her vinner nerdr over oppsettet.
const NERDR: [char; 13] = ['f', 'r', 'w', 's', 'b', 'B', 'd', '?', ':', '(', ')', '[', ']'];

/// Virker fra en control mode-klient, mot panen nerdr viser.
const SENDES: [&str; 39] = [
    "break-pane",
    "clear-history",
    "delete-buffer",
    "display-message",
    "join-pane",
    "kill-pane",
    "kill-window",
    "last-pane",
    "last-window",
    "move-pane",
    "move-window",
    "new-window",
    "next-layout",
    "next-window",
    "paste-buffer",
    "previous-layout",
    "previous-window",
    "rename-session",
    "rename-window",
    "resize-pane",
    "resize-window",
    "respawn-pane",
    "respawn-window",
    "rotate-window",
    "run-shell",
    "select-layout",
    "select-pane",
    "select-window",
    "send-keys",
    "send-prefix",
    "set-buffer",
    "set-environment",
    "set-hook",
    "set-option",
    "set-window-option",
    "source-file",
    "split-window",
    "swap-pane",
    "swap-window",
];

#[derive(Clone, Debug, PartialEq)]
pub enum Hva {
    /// Som tasten etter prefikset i nerdr.
    Som(KeyEvent),
    /// Kommandoene, hver for seg, siden tmux svarer på hver av dem.
    Send(Vec<String>),
}

/// Tasten slik den sammenlignes. Store bokstaver er sin egen tast, så shift
/// teller bare for de andre.
type Nøkkel = (KeyCode, bool, bool, bool);

fn nøkkel(k: &KeyEvent) -> Nøkkel {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let alt = k.modifiers.contains(KeyModifiers::ALT);
    match k.code {
        KeyCode::Char(c) if ctrl => (KeyCode::Char(c.to_ascii_lowercase()), true, alt, false),
        KeyCode::Char(_) => (k.code, false, alt, false),
        _ => (k.code, ctrl, alt, k.modifiers.contains(KeyModifiers::SHIFT)),
    }
}

pub struct Bindinger {
    taster: HashMap<Nøkkel, Hva>,
    gjentas: HashSet<Nøkkel>,
    /// Din tast for nerdr-tasten, til hjelpen og handlingene.
    navn: HashMap<Nøkkel, String>,
    pub gjentatid: Duration,
}

impl Default for Bindinger {
    fn default() -> Bindinger {
        Bindinger { taster: HashMap::new(), gjentas: HashSet::new(), navn: HashMap::new(), gjentatid: Duration::from_millis(500) }
    }
}

impl Bindinger {
    pub fn ny(tabellen: &[Binding]) -> Bindinger {
        let mut b = Bindinger::default();
        for binding in tabellen {
            let Some(k) = tast(&binding.tast) else { continue };
            if binding.gjentas {
                b.gjentas.insert(nøkkel(&k));
            }
            if matches!(nøkkel(&k), (KeyCode::Char(c), false, false, _) if NERDR.contains(&c)) {
                continue;
            }
            let Some(hva) = tolk(&binding.kommando) else { continue };
            if let Hva::Som(nerdr) = &hva
                && nøkkel(nerdr) != nøkkel(&k)
            {
                b.navn.entry(nøkkel(nerdr)).or_insert_with(|| binding.tast.clone());
            }
            b.taster.insert(nøkkel(&k), hva);
        }
        b
    }

    pub fn finn(&self, k: &KeyEvent) -> Option<&Hva> {
        self.taster.get(&nøkkel(k))
    }

    pub fn gjentas(&self, k: &KeyEvent) -> bool {
        self.gjentas.contains(&nøkkel(k))
    }

    fn navn(&self, nerdr: &KeyEvent) -> Option<&str> {
        self.navn.get(&nøkkel(nerdr)).map(String::as_str)
    }

    /// Tasten du har bundet til det nerdr-tasten gjør, ellers nerdr-tasten.
    pub fn vis(&self, nerdr: &KeyEvent) -> String {
        if let Some(navn) = self.navn(nerdr) {
            return navn.to_string();
        }
        let tegn = match nerdr.code {
            KeyCode::Char(' ') => "Space".to_string(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Left => "←".to_string(),
            KeyCode::Up => "↑".to_string(),
            KeyCode::Down => "↓".to_string(),
            KeyCode::Right => "→".to_string(),
            _ => String::new(),
        };
        if nerdr.modifiers.contains(KeyModifiers::CONTROL) { format!("C-{tegn}") } else { tegn }
    }

    /// Om tasten vises med navnet du har bundet.
    pub fn er_din(&self, nerdr: &KeyEvent) -> bool {
        self.navn(nerdr).is_some()
    }
}

/// Fra navnet tmux bruker, som `|`, `C-o`, `M-Left` eller `Space`.
pub fn tast(navn: &str) -> Option<KeyEvent> {
    let mut m = KeyModifiers::NONE;
    let mut rest = navn;
    'ytre: loop {
        for (foran, flagg) in [("C-", KeyModifiers::CONTROL), ("M-", KeyModifiers::ALT), ("S-", KeyModifiers::SHIFT)] {
            if rest.len() > foran.len() && rest.starts_with(foran) {
                m |= flagg;
                rest = &rest[foran.len()..];
                continue 'ytre;
            }
        }
        break;
    }
    let mut tegn = rest.chars();
    let kode = match (tegn.next(), tegn.next()) {
        (Some(c), None) => KeyCode::Char(c),
        _ => match rest {
            "Space" => KeyCode::Char(' '),
            "Up" => KeyCode::Up,
            "Down" => KeyCode::Down,
            "Left" => KeyCode::Left,
            "Right" => KeyCode::Right,
            "Enter" => KeyCode::Enter,
            "Tab" => KeyCode::Tab,
            "BTab" => KeyCode::BackTab,
            "BSpace" => KeyCode::Backspace,
            "Escape" => KeyCode::Esc,
            "Home" => KeyCode::Home,
            "End" => KeyCode::End,
            "PPage" | "PageUp" => KeyCode::PageUp,
            "NPage" | "PageDown" => KeyCode::PageDown,
            "DC" => KeyCode::Delete,
            "IC" => KeyCode::Insert,
            _ => KeyCode::F(rest.strip_prefix('F')?.parse().ok()?),
        },
    };
    Some(KeyEvent::new(kode, m))
}

fn tolk(kommando: &str) -> Option<Hva> {
    let deler = kommandoer(kommando);
    if let [ord] = deler.as_slice()
        && let Some(k) = som(uten_spørsmål(ord))
    {
        return Some(Hva::Som(k));
    }
    let sendes = |ord: &Vec<String>| {
        ord.first().is_some_and(|k| SENDES.contains(&k.as_str())) && !ord.iter().any(|o| o == "{" || o.contains("%%"))
    };
    deler.iter().all(sendes).then(|| Hva::Send(kommando.split(" \\; ").map(|k| k.trim().to_string()).collect()))
}

/// `confirm-before` spør i tmux før kommandoen, men lukking spør ikke i nerdr.
fn uten_spørsmål(ord: &[String]) -> &[String] {
    if ord.first().is_none_or(|k| k != "confirm-before") {
        return ord;
    }
    let mut i = 1;
    while let Some(flagg) = ord.get(i).filter(|o| o.starts_with('-')) {
        i += if matches!(flagg.as_str(), "-p" | "-c") { 2 } else { 1 };
    }
    ord.get(i..).unwrap_or_default()
}

fn som(ord: &[String]) -> Option<KeyEvent> {
    // nerdr starter nye paner og vinduer i mappa til panen uansett.
    let mut uten_mappe = Vec::new();
    let mut ord = ord.iter().map(String::as_str);
    while let Some(o) = ord.next() {
        match o {
            "-c" => {
                if ord.next() != Some("#{pane_current_path}") {
                    return None;
                }
            }
            o => uten_mappe.push(o),
        }
    }
    let med = |kode, m| Some(KeyEvent::new(kode, m));
    let tegn = |c| med(KeyCode::Char(c), KeyModifiers::NONE);
    let retning = |r: &str| match r {
        "-L" => Some(KeyCode::Left),
        "-R" => Some(KeyCode::Right),
        "-U" => Some(KeyCode::Up),
        "-D" => Some(KeyCode::Down),
        _ => None,
    };
    match uten_mappe.as_slice() {
        ["split-window"] | ["split-window", "-v"] => tegn('"'),
        ["split-window", "-h"] => tegn('%'),
        ["new-window"] => tegn('c'),
        ["select-pane", "-t", ":.+"] => tegn('o'),
        ["select-pane", r] => med(retning(r)?, KeyModifiers::NONE),
        ["resize-pane", "-Z"] => tegn('z'),
        ["resize-pane", r] | ["resize-pane", r, "1"] => med(retning(r)?, KeyModifiers::CONTROL),
        ["resize-pane", r, "5"] => med(retning(r)?, KeyModifiers::ALT),
        ["last-pane"] => tegn(';'),
        ["kill-pane"] => tegn('x'),
        ["kill-window"] => tegn('&'),
        ["next-window"] => tegn('n'),
        ["previous-window"] => tegn('p'),
        ["last-window"] => tegn('l'),
        ["swap-pane", "-U"] => tegn('{'),
        ["swap-pane", "-D"] => tegn('}'),
        ["next-layout"] => tegn(' '),
        ["rotate-window"] => med(KeyCode::Char('o'), KeyModifiers::CONTROL),
        ["select-window", "-t", vindu] => {
            let nummer = vindu.strip_prefix(":=").or_else(|| vindu.strip_prefix(':'))?;
            let mut tegn_i = nummer.chars();
            match (tegn_i.next(), tegn_i.next()) {
                (Some(c @ '0'..='9'), None) => tegn(c),
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    fn b(tast: &str, kommando: &str) -> Binding {
        Binding { tast: tast.into(), gjentas: false, kommando: kommando.into() }
    }

    fn k(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn tastene_fra_navnet_i_tmux() {
        assert_eq!(tast("|"), Some(k('|')));
        assert_eq!(tast("-"), Some(k('-')));
        assert_eq!(tast("C-o"), Some(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL)));
        assert_eq!(tast("M-Left"), Some(KeyEvent::new(KeyCode::Left, KeyModifiers::ALT)));
        assert_eq!(tast("Space"), Some(k(' ')));
        assert_eq!(tast("F5"), Some(KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE)));
        assert_eq!(tast("Ukjent"), None);
    }

    #[test]
    fn det_nerdr_har_en_tast_for_blir_den_tasten() {
        let egne = [
            b("|", "split-window -h"),
            b("-", "split-window -v -c \"#{pane_current_path}\""),
            b("h", "select-pane -L"),
            b("H", "resize-pane -L 5"),
            b("X", "confirm-before -p \"kill-pane #P? (y/n)\" kill-pane"),
        ];
        let bindinger = Bindinger::ny(&egne);
        assert_eq!(bindinger.finn(&k('|')), Some(&Hva::Som(k('%'))));
        assert_eq!(bindinger.finn(&k('-')), Some(&Hva::Som(k('"'))));
        assert_eq!(bindinger.finn(&k('h')), Some(&Hva::Som(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))));
        assert_eq!(bindinger.finn(&k('H')), Some(&Hva::Som(KeyEvent::new(KeyCode::Left, KeyModifiers::ALT))));
        assert_eq!(bindinger.finn(&k('X')), Some(&Hva::Som(k('x'))));
        assert_eq!(bindinger.navn(&k('%')), Some("|"));
    }

    #[test]
    fn standarden_i_tmux_er_som_i_nerdr() {
        let bindinger = Bindinger::ny(&[b("%", "split-window -h"), b("&", "confirm-before -p \"kill-window #W? (y/n)\" kill-window")]);
        assert_eq!(bindinger.finn(&k('%')), Some(&Hva::Som(k('%'))));
        assert_eq!(bindinger.finn(&k('&')), Some(&Hva::Som(k('&'))));
        assert_eq!(bindinger.navn(&k('%')), None);
    }

    #[test]
    fn det_som_ikke_trenger_et_skjermbilde_sendes() {
        let egne = [
            b("R", "source-file ~/.tmux.conf \\; display-message \"lastet\""),
            b("g", "display-popup -E -h 18 hopp.sh"),
            b(",", "command-prompt -I \"#W\" { rename-window \"%%\" }"),
        ];
        let bindinger = Bindinger::ny(&egne);
        let send = Hva::Send(vec!["source-file ~/.tmux.conf".into(), "display-message \"lastet\"".into()]);
        assert_eq!(bindinger.finn(&k('R')), Some(&send));
        assert_eq!(bindinger.finn(&k('g')), None);
        assert_eq!(bindinger.finn(&k(',')), None);
    }

    #[test]
    fn nerdr_vinner_på_sine_egne_taster() {
        let bindinger = Bindinger::ny(&[b("r", "source-file ~/.tmux.conf"), b("l", "select-pane -R")]);
        assert_eq!(bindinger.finn(&k('r')), None);
        assert_eq!(bindinger.finn(&k('l')), Some(&Hva::Som(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))));
    }

    #[test]
    fn gjentas_gjelder_tastene_med_r() {
        let bindinger = Bindinger::ny(&[Binding { gjentas: true, ..b("Up", "select-pane -U") }, b("x", "kill-pane")]);
        assert!(bindinger.gjentas(&KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)));
        assert!(!bindinger.gjentas(&k('x')));
    }
}
