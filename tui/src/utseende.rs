//! Hvordan lista og panene ser ut, valgt i utseende-menyen og husket i tmux.

use crate::bokser::Rammer;

/// Global option på tmux-serveren, så valget gjelder alle som kjører nerdr
/// mot den, og overlever omstart.
pub const OPTION: &str = "@nerdr-utseende";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Liste {
    Original,
    Bokser,
    Linjer,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Farge {
    Lite,
    Middels,
    Mye,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Plass {
    Tett,
    Vanlig,
    Romslig,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Utseende {
    pub paner: Rammer,
    pub liste: Liste,
    pub farge: Farge,
    pub plass: Plass,
    pub tid: bool,
}

pub const AKSER: [&str; 5] = ["paner", "lista", "farge", "plass", "tid"];
/// Står i `OPTION` i samme rekkefølge som aksene, og endres ikke.
const VERDIER: [&[&str]; 5] = [
    &["bokser", "linjer"],
    &["original", "bokser", "linjer"],
    &["lite", "middels", "mye"],
    &["tett", "vanlig", "romslig"],
    &["vis", "skjul"],
];
const PANER: [Rammer; 2] = [Rammer::Bokser, Rammer::Linjer];
const LISTER: [Liste; 3] = [Liste::Original, Liste::Bokser, Liste::Linjer];
const FARGER: [Farge; 3] = [Farge::Lite, Farge::Middels, Farge::Mye];
const PLASSER: [Plass; 3] = [Plass::Tett, Plass::Vanlig, Plass::Romslig];
const TIDER: [bool; 2] = [true, false];

impl Utseende {
    pub fn ny(paner: Rammer) -> Utseende {
        Utseende { paner, liste: Liste::Linjer, farge: Farge::Middels, plass: Plass::Vanlig, tid: true }
    }

    pub fn verdier(akse: usize) -> &'static [&'static str] {
        VERDIER[akse]
    }

    /// Verdien som er valgt på aksen, som indeks i `verdier`.
    pub fn indeks(&self, akse: usize) -> usize {
        let i = match akse {
            0 => PANER.iter().position(|p| *p == self.paner),
            1 => LISTER.iter().position(|l| *l == self.liste),
            2 => FARGER.iter().position(|f| *f == self.farge),
            3 => PLASSER.iter().position(|p| *p == self.plass),
            _ => TIDER.iter().position(|t| *t == self.tid),
        };
        i.unwrap_or(0)
    }

    fn sett(&mut self, akse: usize, i: usize) {
        match akse {
            0 => self.paner = PANER[i],
            1 => self.liste = LISTER[i],
            2 => self.farge = FARGER[i],
            3 => self.plass = PLASSER[i],
            _ => self.tid = TIDER[i],
        }
    }

    /// Neste eller forrige verdi på aksen, rundt.
    pub fn endre(&mut self, akse: usize, steg: i32) {
        let antall = VERDIER[akse].len() as i32;
        self.sett(akse, (self.indeks(akse) as i32 + steg).rem_euclid(antall) as usize);
    }

    /// Originalen har ingen farge, plass eller tid å velge.
    pub fn gjelder(&self, akse: usize) -> bool {
        akse < 2 || self.liste != Liste::Original
    }

    pub fn tekst(&self) -> String {
        (0..AKSER.len()).map(|a| VERDIER[a][self.indeks(a)]).collect::<Vec<_>>().join(" ")
    }

    /// Ord som ikke er en verdi på aksen sin, står som før.
    pub fn les(&mut self, tekst: &str) {
        for (akse, ord) in tekst.split_whitespace().enumerate().take(AKSER.len()) {
            if let Some(i) = VERDIER[akse].iter().position(|v| *v == ord) {
                self.sett(akse, i);
            }
        }
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn teksten_leses_tilbake_til_det_samme() {
        let mut u = Utseende::ny(Rammer::Bokser);
        u.endre(1, -1);
        u.endre(2, 1);
        u.endre(4, 1);
        assert_eq!(u.tekst(), "bokser bokser mye vanlig skjul");
        let mut lest = Utseende::ny(Rammer::Linjer);
        lest.les(&u.tekst());
        assert_eq!(lest, u);
    }

    #[test]
    fn ukjente_ord_endrer_ingenting() {
        let mut u = Utseende::ny(Rammer::Linjer);
        u.les("sirkler original");
        assert_eq!((u.paner, u.liste), (Rammer::Linjer, Liste::Original));
    }

    #[test]
    fn endre_går_rundt() {
        let mut u = Utseende::ny(Rammer::Linjer);
        u.endre(0, 1);
        assert_eq!(u.paner, Rammer::Bokser);
        u.endre(3, 2);
        assert_eq!(u.plass, Plass::Tett);
    }
}
