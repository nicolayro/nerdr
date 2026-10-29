//! Hvordan lista og panene ser ut, valgt i utseende-menyen og husket i tmux.

use crate::bokser::Rammer;

/// Global option på tmux-serveren, så valget gjelder alle som kjører nerdr
/// mot den, og overlever omstart.
pub const OPTION: &str = "@nerdr-utseende";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Farge {
    Av,
    /// Tegnene, tiden og rammene i fargen til gruppen.
    På,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Plass {
    Vanlig,
    /// Én linje for hver Claude, uten boks.
    Minimal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Orden {
    /// De aktive øverst.
    Nerdr,
    Mappe,
    /// Den som sist endret tilstand, øverst.
    Aktivitet,
}

/// Hvor de ledige begynner, når lista er sortert prioritert.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ledig {
    Under,
    /// Midt i lista, så de ledige ikke flytter seg når en blir aktiv.
    Midten,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Utseende {
    pub paner: Rammer,
    pub farge: Farge,
    pub plass: Plass,
    pub orden: Orden,
    pub ledig: Ledig,
    /// Linjer til tittelen i boksene.
    pub tittel: usize,
}

/// Står i `OPTION` foran verdien, som `farge=på`, så en akse kan komme til
/// eller forsvinne uten at de andre leses feil.
pub const AKSER: [&str; 6] = ["paner", "farge", "plass", "orden", "ledig", "tittel"];
pub const ORDEN: usize = 3;
const LEDIG: usize = 4;
/// Aksene fra denne gjelder bare boksene.
pub const BOKSAKSER: usize = 5;
/// Står i `OPTION`, og endres ikke.
const VERDIER: [&[&str]; 6] = [
    &["bokser", "linjer"],
    &["av", "på"],
    &["vanlig", "minimal"],
    &["nerdr", "mappe", "aktivitet"],
    &["under", "midten"],
    &["1", "2", "3"],
];
/// Det sorteringen heter i menyen og på knappen, som `VERDIER` for orden.
pub const SORTERINGER: [&str; 3] = ["prioritert", "prosjekt", "aktivitet"];
/// Verdiene som i handlingene, i samme rekkefølge som `VERDIER`.
pub const NAVN: [&[&str]; 6] = [
    &["paner i bokser", "paner med linjer"],
    &["uten farge", "med farge"],
    &["vanlig liste", "minimal liste"],
    &["sorter prioritert", "sorter etter prosjekt", "sorter etter aktivitet"],
    &["ledige under de aktive", "ledige fra midten"],
    &["oppgaven på 1 linje", "oppgaven på 2 linjer", "oppgaven på 3 linjer"],
];
const PANER: [Rammer; 2] = [Rammer::Bokser, Rammer::Linjer];
const FARGER: [Farge; 2] = [Farge::Av, Farge::På];
const PLASSER: [Plass; 2] = [Plass::Vanlig, Plass::Minimal];
const ORDNER: [Orden; 3] = [Orden::Nerdr, Orden::Mappe, Orden::Aktivitet];
const LEDIGE: [Ledig; 2] = [Ledig::Under, Ledig::Midten];

impl Utseende {
    pub fn ny(paner: Rammer) -> Utseende {
        Utseende { paner, farge: Farge::På, plass: Plass::Vanlig, orden: Orden::Nerdr, ledig: Ledig::Under, tittel: 1 }
    }

    /// Navnet på aksen og verdiene i menyen.
    pub fn i_menyen(akse: usize) -> (&'static str, &'static [&'static str]) {
        match akse {
            ORDEN => ("sortering", &SORTERINGER),
            _ => (AKSER[akse], VERDIER[akse]),
        }
    }

    /// Verdien som er valgt på aksen, som indeks i `verdier`.
    pub fn indeks(&self, akse: usize) -> usize {
        let i = match akse {
            0 => PANER.iter().position(|p| *p == self.paner),
            1 => FARGER.iter().position(|f| *f == self.farge),
            2 => PLASSER.iter().position(|p| *p == self.plass),
            3 => ORDNER.iter().position(|o| *o == self.orden),
            4 => LEDIGE.iter().position(|l| *l == self.ledig),
            _ => Some(self.tittel - 1),
        };
        i.unwrap_or(0)
    }

    pub fn sett(&mut self, akse: usize, i: usize) {
        match akse {
            0 => self.paner = PANER[i],
            1 => self.farge = FARGER[i],
            2 => self.plass = PLASSER[i],
            3 => self.orden = ORDNER[i],
            4 => self.ledig = LEDIGE[i],
            _ => self.tittel = i + 1,
        }
    }

    /// Neste eller forrige verdi på aksen, rundt.
    pub fn endre(&mut self, akse: usize, steg: i32) {
        let antall = VERDIER[akse].len() as i32;
        self.sett(akse, (self.indeks(akse) as i32 + steg).rem_euclid(antall) as usize);
    }

    /// Minimal har ingen bokser å velge for, og bare prioritert har ledige.
    pub fn gjelder(&self, akse: usize) -> bool {
        match akse {
            LEDIG => self.orden == Orden::Nerdr,
            _ => akse < BOKSAKSER || self.plass != Plass::Minimal,
        }
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
        u.endre(2, 1);
        u.endre(3, -1);
        assert_eq!(u.tekst(), "paner=bokser farge=av plass=minimal orden=aktivitet ledig=under tittel=1");
        let mut lest = Utseende::ny(Rammer::Linjer);
        lest.les(&u.tekst());
        assert_eq!(lest, u);
    }

    #[test]
    fn ukjente_akser_og_verdier_endrer_ingenting() {
        let mut u = Utseende::ny(Rammer::Linjer);
        u.les("lista=original paner=sirkler bokser farge=av");
        assert_eq!((u.paner, u.farge), (Rammer::Linjer, Farge::Av));
    }

    #[test]
    fn endre_går_rundt() {
        let mut u = Utseende::ny(Rammer::Linjer);
        u.endre(0, 1);
        assert_eq!(u.paner, Rammer::Bokser);
        u.endre(3, 4);
        assert_eq!(u.orden, Orden::Mappe);
    }
}
