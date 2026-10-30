//! Handlingene: alt nerdr kan gjøre, søkbart, med tasten ved siden av, så
//! du kommer i gang uten å kunne tmux og lærer tastene etter hvert.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::bindinger::Bindinger;
use crate::ny;
use crate::utseende::{NAVN, Utseende};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Handling {
    /// Som tasten etter prefikset, med `true` for Ctrl.
    Prefiks(KeyCode, bool),
    /// Aksen og verdien i utseende-menyen.
    Utseende(usize, usize),
}

pub struct Oppføring {
    pub navn: String,
    pub tast: String,
    pub handling: Handling,
}

/// Rekkefølgen er den lista har uten søk, med det du oftest trenger først.
const PREFIKSTASTER: [(KeyCode, bool, &str); 24] = [
    (KeyCode::Char('f'), false, "ny sesjon"),
    (KeyCode::Char('r'), false, "hent samtale"),
    (KeyCode::Char('c'), false, "ny terminal"),
    (KeyCode::Char('%'), false, "del side om side"),
    (KeyCode::Char('"'), false, "del over og under"),
    (KeyCode::Char('w'), false, "tilbake til agentlista"),
    (KeyCode::Char(')'), false, "neste sesjon"),
    (KeyCode::Char('('), false, "forrige sesjon"),
    (KeyCode::Char('n'), false, "neste vindu"),
    (KeyCode::Char('p'), false, "forrige vindu"),
    (KeyCode::Char('l'), false, "siste vindu"),
    (KeyCode::Char('o'), false, "neste pane"),
    (KeyCode::Char(';'), false, "siste pane"),
    (KeyCode::Char('z'), false, "zoom pane"),
    (KeyCode::Char('['), false, "kopimodus"),
    (KeyCode::Char('{'), false, "flytt pane bakover"),
    (KeyCode::Char('}'), false, "flytt pane framover"),
    (KeyCode::Char(' '), false, "neste oppsett"),
    (KeyCode::Char('o'), true, "roter paner"),
    (KeyCode::Char('x'), false, "lukk pane"),
    (KeyCode::Char('&'), false, "lukk vindu"),
    (KeyCode::Char('b'), false, "innstillinger"),
    (KeyCode::Char('?'), false, "hjelp"),
    (KeyCode::Char('d'), false, "avslutt"),
];

pub struct Handlinger {
    alle: Vec<Oppføring>,
    pub søk: String,
    pub valgt: usize,
    /// Første treff som vises.
    pub rull: usize,
}

impl Handlinger {
    pub fn ny(prefiks: &str, bindinger: &Bindinger, utseende: &Utseende) -> Handlinger {
        let mut alle: Vec<Oppføring> = PREFIKSTASTER
            .iter()
            .map(|&(kode, ctrl, navn)| {
                let m = if ctrl { KeyModifiers::CONTROL } else { KeyModifiers::NONE };
                Oppføring {
                    navn: navn.into(),
                    tast: format!("{prefiks} {}", bindinger.vis(&KeyEvent::new(kode, m))),
                    handling: Handling::Prefiks(kode, ctrl),
                }
            })
            .collect();
        for (akse, navn) in NAVN.iter().enumerate() {
            for (i, navn) in navn.iter().enumerate() {
                let valgt = utseende.gjelder(akse) && utseende.indeks(akse) == i;
                alle.push(Oppføring {
                    navn: (*navn).into(),
                    tast: if valgt { "✓".into() } else { String::new() },
                    handling: Handling::Utseende(akse, i),
                });
            }
        }
        Handlinger { alle, søk: String::new(), valgt: 0, rull: 0 }
    }

    pub fn antall(&self) -> usize {
        self.alle.len()
    }

    /// Oppføringene søket treffer, best først, med tegnindeksene det traff.
    /// Like gode beholder rekkefølgen.
    pub fn treff(&self) -> Vec<(&Oppføring, Vec<usize>)> {
        let mut treff: Vec<(i32, &Oppføring, Vec<usize>)> = self
            .alle
            .iter()
            .filter_map(|k| {
                let (poeng, tegn) = ny::poeng(&self.søk, &k.navn)?;
                Some((poeng, k, tegn))
            })
            .collect();
        treff.sort_by_key(|(poeng, ..)| -poeng);
        treff.into_iter().map(|(_, k, tegn)| (k, tegn)).collect()
    }

    pub fn valgt(&self) -> Option<Handling> {
        self.treff().get(self.valgt).map(|(k, _)| k.handling)
    }

    pub fn skriv(&mut self, tekst: &str) {
        self.søk.extend(tekst.chars().filter(|c| !c.is_control()));
        self.valgt = 0;
    }

    pub fn slett(&mut self) {
        self.søk.pop();
        self.valgt = 0;
    }

    pub fn tøm(&mut self) {
        self.søk.clear();
        self.valgt = 0;
    }

    pub fn flytt(&mut self, steg: isize) {
        let siste = self.treff().len().saturating_sub(1) as isize;
        self.valgt = (self.valgt as isize + steg).clamp(0, siste) as usize;
    }
}

#[cfg(test)]
mod tester {
    use super::*;
    use crate::bokser::Rammer;

    #[test]
    fn søket_finner_handlingen_med_tasten() {
        let mut h = Handlinger::ny("C-b", &Bindinger::default(), &Utseende::ny(Rammer::Linjer));
        h.skriv("zoom");
        let (k, _) = &h.treff()[0];
        assert_eq!((k.navn.as_str(), k.tast.as_str()), ("zoom pane", "C-b z"));
        h.tøm();
        h.skriv("sesjon");
        assert_eq!(h.valgt(), Some(Handling::Prefiks(KeyCode::Char('f'), false)));
    }

    #[test]
    fn utseendet_som_er_valgt_står_som_valgt() {
        let h = Handlinger::ny("C-b", &Bindinger::default(), &Utseende::ny(Rammer::Linjer));
        let linjer = h.alle.iter().find(|k| k.navn == "paner med linjer").unwrap();
        assert_eq!(linjer.tast, "✓");
        assert_eq!(h.alle.iter().find(|k| k.navn == "paner i bokser").unwrap().tast, "");
    }

    #[test]
    fn hver_verdi_i_utseendet_har_et_navn() {
        for (akse, navn) in NAVN.iter().enumerate() {
            assert_eq!(navn.len(), Utseende::i_menyen(akse).1.len(), "{}", crate::utseende::AKSER[akse]);
        }
    }
}
