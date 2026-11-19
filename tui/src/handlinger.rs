//! Handlingene: alt nerdr kan gjøre, søkbart, med tasten ved siden av, så
//! du kommer i gang uten å kunne tmux og lærer tastene etter hvert.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tmuxtaster::Bindinger;
use crate::ny;
use crate::popup::Søkefelt;
use crate::utseende::{Akse, Utseende};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Handling {
    /// Som tasten etter prefikset, med `true` for Ctrl.
    Prefiks(KeyCode, bool),
    /// Aksen og verdien i utseende-menyen.
    Utseende(Akse, usize),
}

pub struct Oppføring {
    pub navn: String,
    pub tast: String,
    pub handling: Handling,
}

/// Rekkefølgen er den lista har uten søk, med det du oftest trenger først.
const PREFIKSTASTER: [(KeyCode, bool, &str); 26] = [
    (KeyCode::Char('f'), false, "ny sesjon"),
    (KeyCode::Char('F'), false, "ny chat"),
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
    (KeyCode::Char('b'), false, "agentlista som stripe"),
    (KeyCode::Char('u'), false, "innstillinger"),
    (KeyCode::Char('?'), false, "hjelp"),
    (KeyCode::Char('d'), false, "avslutt"),
];

pub struct Handlinger {
    alle: Vec<Oppføring>,
    pub felt: Søkefelt,
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
        for akse in Akse::ALLE {
            for (i, verdi) in akse.verdier().iter().enumerate() {
                let valgt = utseende.gjelder(akse) && utseende.indeks(akse) == i;
                alle.push(Oppføring {
                    navn: verdi.navn.into(),
                    tast: if valgt { "✓".into() } else { String::new() },
                    handling: Handling::Utseende(akse, i),
                });
            }
        }
        Handlinger { alle, felt: Søkefelt::default() }
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
                let (poeng, tegn) = ny::poeng(&self.felt.søk, &k.navn)?;
                Some((poeng, k, tegn))
            })
            .collect();
        treff.sort_by_key(|(poeng, ..)| -poeng);
        treff.into_iter().map(|(_, k, tegn)| (k, tegn)).collect()
    }

    pub fn valgt(&self) -> Option<Handling> {
        self.treff().get(self.felt.valgt).map(|(k, _)| k.handling)
    }
}

#[cfg(test)]
mod tester {
    use super::*;
    use crate::bokser::Rammer;

    #[test]
    fn søket_finner_handlingen_med_tasten() {
        let mut h = Handlinger::ny("C-b", &Bindinger::default(), &Utseende::ny(Rammer::Linjer));
        h.felt.skriv("zoom");
        let (k, _) = &h.treff()[0];
        assert_eq!((k.navn.as_str(), k.tast.as_str()), ("zoom pane", "C-b z"));
        h.felt.søk.clear();
        h.felt.skriv("sesjon");
        assert_eq!(h.valgt(), Some(Handling::Prefiks(KeyCode::Char('f'), false)));
    }

    #[test]
    fn utseendet_som_er_valgt_står_som_valgt() {
        let h = Handlinger::ny("C-b", &Bindinger::default(), &Utseende::ny(Rammer::Linjer));
        let linjer = h.alle.iter().find(|k| k.navn == "paner med linjer").unwrap();
        assert_eq!(linjer.tast, "✓");
        assert_eq!(h.alle.iter().find(|k| k.navn == "paner i bokser").unwrap().tast, "");
    }
}
