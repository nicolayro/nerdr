//! Det nerdr viser over resten og tar tastene til. Bare én om gangen, så
//! rekkefølgen mellom dem kan ikke bli ulik på ulike steder.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::Lukke;
use crate::handlinger::Handlinger;
use crate::ny::Velger;
use crate::utseende::Akse;

pub enum Popup {
    Hjelp(u16),
    Utseende(Akse),
    Lukke(Lukke),
    Handlinger(Handlinger),
    Velger(Velger),
}

/// Søket, det valgte treffet og hvor langt lista er rullet, i handlingene og
/// velgeren.
#[derive(Default)]
pub struct Søkefelt {
    pub søk: String,
    pub valgt: usize,
    /// Første treff som vises.
    pub rull: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Søketast {
    Lukk,
    Velg,
    /// Bytter mellom mapper og samtaler i velgeren.
    Bytt,
    /// Søket er endret, så treffene må regnes ut på nytt.
    Endret,
    Flyttet,
    Ubrukt,
}

impl Søkefelt {
    pub fn skriv(&mut self, tekst: &str) {
        self.søk.extend(tekst.chars().filter(|c| !c.is_control()));
        self.valgt = 0;
    }

    pub fn flytt(&mut self, steg: isize, antall: usize) {
        let siste = antall.saturating_sub(1) as isize;
        self.valgt = (self.valgt as isize + steg).clamp(0, siste) as usize;
    }

    /// `side` er antall rader en sidetast flytter, og `antall` treffene søket har.
    fn tast(&mut self, k: KeyEvent, side: isize, antall: usize) -> Søketast {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let flytt = |felt: &mut Søkefelt, steg| {
            felt.flytt(steg, antall);
            Søketast::Flyttet
        };
        match k.code {
            KeyCode::Esc => Søketast::Lukk,
            KeyCode::Char('c') if ctrl => Søketast::Lukk,
            KeyCode::Enter => Søketast::Velg,
            KeyCode::Char('r') if ctrl => Søketast::Bytt,
            KeyCode::PageUp => flytt(self, -side),
            KeyCode::PageDown => flytt(self, side),
            KeyCode::Up => flytt(self, -1),
            KeyCode::Char('p' | 'k') if ctrl => flytt(self, -1),
            KeyCode::Down => flytt(self, 1),
            KeyCode::Char('n' | 'j') if ctrl => flytt(self, 1),
            KeyCode::Char('u') if ctrl => {
                self.søk.clear();
                self.valgt = 0;
                Søketast::Endret
            }
            KeyCode::Backspace => {
                self.søk.pop();
                self.valgt = 0;
                Søketast::Endret
            }
            KeyCode::Char(c) if !ctrl => {
                self.skriv(c.encode_utf8(&mut [0; 4]));
                Søketast::Endret
            }
            _ => Søketast::Ubrukt,
        }
    }
}

impl Popup {
    pub fn felt(&mut self) -> Option<&mut Søkefelt> {
        match self {
            Popup::Handlinger(h) => Some(&mut h.felt),
            Popup::Velger(v) => Some(&mut v.felt),
            _ => None,
        }
    }

    pub fn antall_treff(&self) -> usize {
        match self {
            Popup::Handlinger(h) => h.treff().len(),
            Popup::Velger(v) => v.antall_treff(),
            _ => 0,
        }
    }

    /// Velgeren regner ut treffene når søket endres, handlingene hver gang de vises.
    fn søket_endret(&mut self) {
        if let Popup::Velger(v) = self {
            v.finn_treff();
        }
    }

    pub fn søketast(&mut self, k: KeyEvent, side: isize) -> Søketast {
        let antall = self.antall_treff();
        let Some(felt) = self.felt() else { return Søketast::Ubrukt };
        let tast = felt.tast(k, side, antall);
        match (tast, &mut *self) {
            (Søketast::Endret, _) => self.søket_endret(),
            (Søketast::Bytt, Popup::Velger(v)) => v.bytt(),
            (Søketast::Bytt, _) => return Søketast::Ubrukt,
            _ => {}
        }
        tast
    }

    pub fn lim_inn(&mut self, tekst: &str) -> bool {
        let Some(felt) = self.felt() else { return false };
        felt.skriv(tekst);
        self.søket_endret();
        true
    }

    pub fn flytt(&mut self, steg: isize) {
        let antall = self.antall_treff();
        if let Some(felt) = self.felt() {
            felt.flytt(steg, antall);
        }
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    fn tast(felt: &mut Søkefelt, kode: KeyCode, mods: KeyModifiers) -> Søketast {
        felt.tast(KeyEvent::new(kode, mods), 5, 10)
    }

    #[test]
    fn å_skrive_og_slette_endrer_søket_og_nullstiller_valget() {
        let mut felt = Søkefelt { valgt: 3, ..Søkefelt::default() };
        assert_eq!(tast(&mut felt, KeyCode::Char('a'), KeyModifiers::NONE), Søketast::Endret);
        assert_eq!((felt.søk.as_str(), felt.valgt), ("a", 0));
        felt.valgt = 2;
        assert_eq!(tast(&mut felt, KeyCode::Backspace, KeyModifiers::NONE), Søketast::Endret);
        assert_eq!((felt.søk.as_str(), felt.valgt), ("", 0));
        felt.skriv("abc");
        assert_eq!(tast(&mut felt, KeyCode::Char('u'), KeyModifiers::CONTROL), Søketast::Endret);
        assert_eq!(felt.søk, "");
    }

    #[test]
    fn valget_stopper_i_endene() {
        let mut felt = Søkefelt::default();
        tast(&mut felt, KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(felt.valgt, 0);
        tast(&mut felt, KeyCode::PageDown, KeyModifiers::NONE);
        tast(&mut felt, KeyCode::PageDown, KeyModifiers::NONE);
        tast(&mut felt, KeyCode::PageDown, KeyModifiers::NONE);
        assert_eq!(felt.valgt, 9);
    }

    #[test]
    fn esc_og_ctrl_c_lukker_og_andre_taster_med_ctrl_gjør_ingenting() {
        let mut felt = Søkefelt::default();
        assert_eq!(tast(&mut felt, KeyCode::Esc, KeyModifiers::NONE), Søketast::Lukk);
        assert_eq!(tast(&mut felt, KeyCode::Char('c'), KeyModifiers::CONTROL), Søketast::Lukk);
        assert_eq!(tast(&mut felt, KeyCode::Char('x'), KeyModifiers::CONTROL), Søketast::Ubrukt);
        assert_eq!(felt.søk, "");
    }
}
