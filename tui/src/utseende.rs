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

/// Hvor de ledige begynner, når lista er ordnet som nerdr.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ledig {
    Under,
    /// Midt i lista, så de ledige ikke flytter seg når en blir aktiv.
    Midten,
}

/// Midlertidig, til vi har valgt hvordan prosjektet står over kortene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Topp {
    /// En rad med bakgrunn.
    Rad,
    /// En strek fra navnet til skillelinja.
    Strek,
}

/// Midlertidig, til vi har valgt fargen til det som er ferdig og ikke sett.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ulestfarge {
    /// Som det som spør.
    Oransje,
    Blå,
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
    pub topp: Topp,
    pub ulest: Ulestfarge,
}

/// Står i `OPTION` foran verdien, som `farge=på`, så en akse kan komme til
/// eller forsvinne uten at de andre leses feil.
pub const AKSER: [&str; 8] = ["paner", "farge", "plass", "orden", "ledig", "tittel", "topp", "ulest"];
const LEDIG: usize = 4;
/// Aksene fra denne gjelder bare boksene.
pub const BOKSAKSER: usize = 5;
/// Aksene fra denne prøves ut, og står for seg nederst i menyen.
pub const MIDLERTIDIGE: usize = 6;
const TOPP: usize = 6;
const ULEST: usize = 7;
/// Står i `OPTION`, og endres ikke.
const VERDIER: [&[&str]; 8] = [
    &["bokser", "linjer"],
    &["av", "på"],
    &["vanlig", "minimal"],
    &["nerdr", "mappe", "aktivitet"],
    &["under", "midten"],
    &["1", "2", "3"],
    &["rad", "strek"],
    &["oransje", "blå"],
];
/// Verdiene som i handlingene, i samme rekkefølge som `VERDIER`.
pub const NAVN: [&[&str]; 8] = [
    &["paner i bokser", "paner med linjer"],
    &["uten farge", "med farge"],
    &["vanlig liste", "minimal liste"],
    &["ordne som nerdr", "ordne etter mappe", "ordne etter aktivitet"],
    &["ledige under de aktive", "ledige fra midten"],
    &["oppgaven på 1 linje", "oppgaven på 2 linjer", "oppgaven på 3 linjer"],
    &["prosjektet på en rad", "prosjektet med strek"],
    &["ulest i oransje", "ulest i blått"],
];
const PANER: [Rammer; 2] = [Rammer::Bokser, Rammer::Linjer];
const FARGER: [Farge; 2] = [Farge::Av, Farge::På];
const PLASSER: [Plass; 2] = [Plass::Vanlig, Plass::Minimal];
const ORDNER: [Orden; 3] = [Orden::Nerdr, Orden::Mappe, Orden::Aktivitet];
const LEDIGE: [Ledig; 2] = [Ledig::Under, Ledig::Midten];
const TOPPER: [Topp; 2] = [Topp::Rad, Topp::Strek];
const ULESTFARGER: [Ulestfarge; 2] = [Ulestfarge::Oransje, Ulestfarge::Blå];

impl Utseende {
    pub fn ny(paner: Rammer) -> Utseende {
        Utseende {
            paner,
            farge: Farge::På,
            plass: Plass::Vanlig,
            orden: Orden::Nerdr,
            ledig: Ledig::Under,
            tittel: 1,
            topp: Topp::Rad,
            ulest: Ulestfarge::Oransje,
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
            3 => ORDNER.iter().position(|o| *o == self.orden),
            4 => LEDIGE.iter().position(|l| *l == self.ledig),
            5 => Some(self.tittel - 1),
            6 => TOPPER.iter().position(|t| *t == self.topp),
            _ => ULESTFARGER.iter().position(|f| *f == self.ulest),
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
            5 => self.tittel = i + 1,
            6 => self.topp = TOPPER[i],
            _ => self.ulest = ULESTFARGER[i],
        }
    }

    /// Neste eller forrige verdi på aksen, rundt.
    pub fn endre(&mut self, akse: usize, steg: i32) {
        let antall = VERDIER[akse].len() as i32;
        self.sett(akse, (self.indeks(akse) as i32 + steg).rem_euclid(antall) as usize);
    }

    /// Minimal har ingen bokser å velge for, og bare nerdr har ledige.
    pub fn gjelder(&self, akse: usize) -> bool {
        match akse {
            LEDIG => self.orden == Orden::Nerdr,
            TOPP => self.orden == Orden::Mappe,
            ULEST => self.farge == Farge::På,
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
        assert_eq!(u.tekst(), "paner=bokser farge=av plass=minimal orden=aktivitet ledig=under tittel=1 topp=rad ulest=oransje");
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
