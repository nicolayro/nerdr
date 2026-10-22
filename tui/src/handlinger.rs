//! Kommandopaletten: alt nerdr kan gjøre, søkbart, med tasten ved siden av, så
//! du kommer i gang uten å kunne tmux og lærer tastene etter hvert.

use ratatui::crossterm::event::KeyCode;

use crate::meny::Valg;
use crate::ny;
use crate::utseende::{NAVN, Utseende};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Handling {
    /// Som valget i «Ny»-menyen.
    Ny(Valg),
    /// Som tasten etter prefikset, med `true` for Ctrl.
    Prefiks(KeyCode, bool),
    /// Aksen og verdien i utseende-menyen.
    Utseende(usize, usize),
}

pub struct Kommando {
    pub navn: String,
    pub tast: String,
    pub handling: Handling,
}

/// Rekkefølgen er den lista har uten søk, med det du oftest trenger først.
const PREFIKSTASTER: [(KeyCode, bool, &str); 18] = [
    (KeyCode::Char('w'), false, "tilbake til lista"),
    (KeyCode::Char(')'), false, "neste Claude i lista"),
    (KeyCode::Char('('), false, "forrige Claude i lista"),
    (KeyCode::Char('n'), false, "neste vindu"),
    (KeyCode::Char('p'), false, "forrige vindu"),
    (KeyCode::Char('l'), false, "vinduet du var i sist"),
    (KeyCode::Char('o'), false, "neste pane"),
    (KeyCode::Char(';'), false, "forrige aktive pane"),
    (KeyCode::Char('z'), false, "zoom inn og ut på panen"),
    (KeyCode::Char('{'), false, "bytt plass med panen før"),
    (KeyCode::Char('}'), false, "bytt plass med panen etter"),
    (KeyCode::Char(' '), false, "neste oppsett for panene"),
    (KeyCode::Char('o'), true, "roter panene"),
    (KeyCode::Char('x'), false, "lukk panen"),
    (KeyCode::Char('&'), false, "lukk vinduet"),
    (KeyCode::Char('b'), false, "velg utseende"),
    (KeyCode::Char('?'), false, "hjelp"),
    (KeyCode::Char('d'), false, "avslutt nerdr"),
];

pub struct Handlinger {
    kommandoer: Vec<Kommando>,
    pub søk: String,
    pub valgt: usize,
}

impl Handlinger {
    /// `ny` er valgene «Ny»-menyen har der du er.
    pub fn ny(prefiks: &str, ny: &[Valg], utseende: &Utseende) -> Handlinger {
        let mut kommandoer: Vec<Kommando> = ny
            .iter()
            .map(|&valg| Kommando { navn: navn(valg).into(), tast: format!("{prefiks} + {}", valg.tast()), handling: Handling::Ny(valg) })
            .collect();
        kommandoer.extend(PREFIKSTASTER.iter().map(|&(kode, ctrl, navn)| Kommando {
            navn: navn.into(),
            tast: format!("{prefiks} {}", tast(kode, ctrl)),
            handling: Handling::Prefiks(kode, ctrl),
        }));
        for (akse, navn) in NAVN.iter().enumerate() {
            for (i, navn) in navn.iter().enumerate() {
                let valgt = utseende.gjelder(akse) && utseende.indeks(akse) == i;
                kommandoer.push(Kommando {
                    navn: (*navn).into(),
                    tast: if valgt { "valgt".into() } else { String::new() },
                    handling: Handling::Utseende(akse, i),
                });
            }
        }
        Handlinger { kommandoer, søk: String::new(), valgt: 0 }
    }

    pub fn antall(&self) -> usize {
        self.kommandoer.len()
    }

    /// Kommandoene søket treffer, best først, med tegnindeksene det traff.
    /// Like gode beholder rekkefølgen.
    pub fn treff(&self) -> Vec<(&Kommando, Vec<usize>)> {
        let mut treff: Vec<(i32, &Kommando, Vec<usize>)> = self
            .kommandoer
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

/// Teksten i «Ny»-menyen står under overskriften «ny», og trenger ordet her.
fn navn(valg: Valg) -> &'static str {
    match valg {
        Valg::Mappe => "ny Claude i en mappe",
        Valg::Claude => "ny Claude her",
        Valg::Worktree => "ny Claude i en ny worktree",
        Valg::Fortsett => "fortsett en samtale",
        Valg::Terminal => "ny terminal her",
        Valg::SideOmSide => "del side om side",
        Valg::OverOgUnder => "del over og under",
    }
}

fn tast(kode: KeyCode, ctrl: bool) -> String {
    let tegn = match kode {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        _ => String::new(),
    };
    if ctrl { format!("C-{tegn}") } else { tegn }
}

#[cfg(test)]
mod tester {
    use super::*;
    use crate::bokser::Rammer;

    #[test]
    fn søket_finner_kommandoen_med_tasten() {
        let mut h = Handlinger::ny("C-b", &[Valg::Mappe, Valg::Fortsett], &Utseende::ny(Rammer::Linjer));
        h.skriv("zoom");
        let (k, _) = &h.treff()[0];
        assert_eq!((k.navn.as_str(), k.tast.as_str()), ("zoom inn og ut på panen", "C-b z"));
        h.tøm();
        h.skriv("mappe");
        assert_eq!(h.valgt(), Some(Handling::Ny(Valg::Mappe)));
    }

    #[test]
    fn utseendet_som_er_valgt_står_som_valgt() {
        let h = Handlinger::ny("C-b", &[], &Utseende::ny(Rammer::Linjer));
        let linjer = h.kommandoer.iter().find(|k| k.navn == "paner med linjer").unwrap();
        assert_eq!(linjer.tast, "valgt");
        assert_eq!(h.kommandoer.iter().find(|k| k.navn == "paner i bokser").unwrap().tast, "");
    }

    #[test]
    fn hver_verdi_i_utseendet_har_et_navn() {
        for (akse, navn) in NAVN.iter().enumerate() {
            assert_eq!(navn.len(), Utseende::verdier(akse).len(), "{}", crate::utseende::AKSER[akse]);
        }
    }
}
