//! Hvordan lista og panene ser ut, valgt i utseende-menyen og husket i tmux.

use crate::bokser::Rammer;

/// Global option på tmux-serveren, så valget gjelder alle som kjører nerdr
/// mot den, og overlever omstart.
pub const OPTION: &str = "@nerdr-utseende";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Farge {
    Lite,
    Middels,
    Mye,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Plass {
    /// Én linje for hver Claude, uten boks.
    Tett,
    Vanlig,
    Romslig,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Orden {
    /// Det som venter på deg, øverst.
    Nerdr,
    Mappe,
    /// Den som sist endret tilstand, øverst.
    Aktivitet,
}

/// Hvordan prosjektet i toppkanten skiller seg ut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Uthev {
    Ingen,
    Fet,
    /// I fargen til køen.
    Farge,
}

/// Hvor glyfen står i toppkanten.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Symbol {
    /// Foran prosjektet.
    Venstre,
    /// Etter tiden.
    Høyre,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Utseende {
    pub paner: Rammer,
    pub farge: Farge,
    pub plass: Plass,
    pub tid: bool,
    pub orden: Orden,
    /// Linjer til tittelen i boksene.
    pub tittel: usize,
    pub uthev: Uthev,
    pub symbol: Symbol,
}

/// Står i `OPTION` foran verdien, som `farge=middels`, så en akse kan komme
/// til eller forsvinne uten at de andre leses feil.
pub const AKSER: [&str; 8] = ["paner", "farge", "plass", "tid", "orden", "tittel", "uthev", "symbol"];
/// Aksene fra denne gjelder bare boksene.
pub const BOKSAKSER: usize = 5;
/// Står i `OPTION`, og endres ikke.
const VERDIER: [&[&str]; 8] = [
    &["bokser", "linjer"],
    &["lite", "middels", "mye"],
    &["tett", "vanlig", "romslig"],
    &["vis", "skjul"],
    &["nerdr", "mappe", "aktivitet"],
    &["1", "2", "3"],
    &["ingen", "fet", "farge"],
    &["venstre", "høyre"],
];
const PANER: [Rammer; 2] = [Rammer::Bokser, Rammer::Linjer];
const FARGER: [Farge; 3] = [Farge::Lite, Farge::Middels, Farge::Mye];
const PLASSER: [Plass; 3] = [Plass::Tett, Plass::Vanlig, Plass::Romslig];
const TIDER: [bool; 2] = [true, false];
const ORDNER: [Orden; 3] = [Orden::Nerdr, Orden::Mappe, Orden::Aktivitet];
const UTHEVINGER: [Uthev; 3] = [Uthev::Ingen, Uthev::Fet, Uthev::Farge];
const SYMBOLER: [Symbol; 2] = [Symbol::Venstre, Symbol::Høyre];

impl Utseende {
    pub fn ny(paner: Rammer) -> Utseende {
        Utseende {
            paner,
            farge: Farge::Middels,
            plass: Plass::Vanlig,
            tid: true,
            orden: Orden::Nerdr,
            tittel: 1,
            uthev: Uthev::Fet,
            symbol: Symbol::Venstre,
        }
    }

    pub fn verdier(akse: usize) -> &'static [&'static str] {
        VERDIER[akse]
    }

    /// Verdien som er valgt på aksen, som indeks i `verdier`.
    pub fn indeks(&self, akse: usize) -> usize {
        let i = match akse {
            0 => PANER.iter().position(|p| *p == self.paner),
            1 => FARGER.iter().position(|f| *f == self.farge),
            2 => PLASSER.iter().position(|p| *p == self.plass),
            3 => TIDER.iter().position(|t| *t == self.tid),
            4 => ORDNER.iter().position(|o| *o == self.orden),
            5 => Some(self.tittel - 1),
            6 => UTHEVINGER.iter().position(|u| *u == self.uthev),
            _ => SYMBOLER.iter().position(|s| *s == self.symbol),
        };
        i.unwrap_or(0)
    }

    fn sett(&mut self, akse: usize, i: usize) {
        match akse {
            0 => self.paner = PANER[i],
            1 => self.farge = FARGER[i],
            2 => self.plass = PLASSER[i],
            3 => self.tid = TIDER[i],
            4 => self.orden = ORDNER[i],
            5 => self.tittel = i + 1,
            6 => self.uthev = UTHEVINGER[i],
            _ => self.symbol = SYMBOLER[i],
        }
    }

    /// Neste eller forrige verdi på aksen, rundt.
    pub fn endre(&mut self, akse: usize, steg: i32) {
        let antall = VERDIER[akse].len() as i32;
        self.sett(akse, (self.indeks(akse) as i32 + steg).rem_euclid(antall) as usize);
    }

    /// Tett har ingen bokser å velge for.
    pub fn gjelder(&self, akse: usize) -> bool {
        akse < BOKSAKSER || self.plass != Plass::Tett
    }

    pub fn tekst(&self) -> String {
        (0..AKSER.len()).map(|a| format!("{}={}", AKSER[a], VERDIER[a][self.indeks(a)])).collect::<Vec<_>>().join(" ")
    }

    /// Akser og verdier nerdr ikke kjenner, endrer ingenting.
    pub fn les(&mut self, tekst: &str) {
        for (navn, verdi) in tekst.split_whitespace().filter_map(|o| o.split_once('=')) {
            if let Some(akse) = AKSER.iter().position(|a| *a == navn)
                && let Some(i) = VERDIER[akse].iter().position(|v| *v == verdi)
            {
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
        u.endre(1, 1);
        u.endre(3, 1);
        u.endre(4, -1);
        assert_eq!(
            u.tekst(),
            "paner=bokser farge=mye plass=vanlig tid=skjul orden=aktivitet tittel=1 uthev=fet symbol=venstre"
        );
        let mut lest = Utseende::ny(Rammer::Linjer);
        lest.les(&u.tekst());
        assert_eq!(lest, u);
    }

    #[test]
    fn ukjente_akser_og_verdier_endrer_ingenting() {
        let mut u = Utseende::ny(Rammer::Linjer);
        u.les("lista=original paner=sirkler bokser farge=lite");
        assert_eq!((u.paner, u.farge), (Rammer::Linjer, Farge::Lite));
    }

    #[test]
    fn endre_går_rundt() {
        let mut u = Utseende::ny(Rammer::Linjer);
        u.endre(0, 1);
        assert_eq!(u.paner, Rammer::Bokser);
        u.endre(2, 2);
        assert_eq!(u.plass, Plass::Tett);
    }
}
