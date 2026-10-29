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

/// Midlertidig, til vi har valgt rekkefølgen på prosjektene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Prosjekter {
    /// Etter den eldste panen i prosjektet, så et nytt prosjekt kommer nederst.
    Startet,
    Alfabetisk,
}

/// Midlertidig, til vi har valgt hvordan kortene ser ut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kort {
    Bokser,
    /// Prosjektet er rammen, og hver Claude en del av den.
    Ramme,
    /// Fargen i en kant til venstre, uten boks.
    Kant,
}

/// Midlertidig, til vi har valgt hvordan prosjektet står over kortene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Topp {
    Tekst,
    /// En rad med bakgrunn.
    Rad,
    /// En strek fra navnet til skillelinja, uten bakgrunn.
    Strek,
}

/// Midlertidig, til vi har valgt hvordan den valgte vises.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Valgt {
    Bakgrunn,
    /// Bakgrunn og tykke streker i hele fargen.
    Tykk,
    /// Rammen i hele fargen og tittelen fet, uten bakgrunn.
    Ramme,
}

/// Midlertidig, til vi har valgt fargen til det som er ferdig og ikke sett.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ulestfarge {
    /// Som det som spør.
    Oransje,
    Blå,
}

/// Midlertidig, til vi har valgt hvor stien til prosjektet står.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sti {
    Av,
    /// Mappa repoet ligger i, etter navnet.
    Etter,
    /// Hele stien, med navnet sist.
    Før,
    /// Hele stien på linja under navnet.
    Under,
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
    pub prosjekter: Prosjekter,
    pub kort: Kort,
    pub topp: Topp,
    pub valgt: Valgt,
    pub ulest: Ulestfarge,
    pub sti: Sti,
}

/// Står i `OPTION` foran verdien, som `farge=på`, så en akse kan komme til
/// eller forsvinne uten at de andre leses feil.
pub const AKSER: [&str; 12] =
    ["paner", "farge", "plass", "orden", "ledig", "tittel", "mapper", "kort", "topp", "valgt", "ulest", "sti"];
const LEDIG: usize = 4;
/// Aksene fra denne gjelder bare boksene.
pub const BOKSAKSER: usize = 5;
/// Aksene fra denne prøves ut, og står for seg nederst i menyen.
pub const MIDLERTIDIGE: usize = 6;
const KORT: usize = 7;
const TOPP: usize = 8;
const VALGT: usize = 9;
const ULEST: usize = 10;
const STI: usize = 11;
/// Står i `OPTION`, og endres ikke.
const VERDIER: [&[&str]; 12] = [
    &["bokser", "linjer"],
    &["av", "på"],
    &["vanlig", "minimal"],
    &["nerdr", "mappe", "aktivitet"],
    &["under", "midten"],
    &["1", "2", "3"],
    &["startet", "alfabetisk"],
    &["bokser", "ramme", "kant"],
    &["tekst", "rad", "strek"],
    &["bakgrunn", "tykk", "ramme"],
    &["oransje", "blå"],
    &["av", "etter", "før", "under"],
];
/// Verdiene som i handlingene, i samme rekkefølge som `VERDIER`.
pub const NAVN: [&[&str]; 12] = [
    &["paner i bokser", "paner med linjer"],
    &["uten farge", "med farge"],
    &["vanlig liste", "minimal liste"],
    &["ordne som nerdr", "ordne etter mappe", "ordne etter aktivitet"],
    &["ledige under de aktive", "ledige fra midten"],
    &["oppgaven på 1 linje", "oppgaven på 2 linjer", "oppgaven på 3 linjer"],
    &["prosjektene etter når de startet", "prosjektene alfabetisk"],
    &["kort i bokser", "prosjektet som ramme", "fargen i en kant"],
    &["prosjektet som tekst", "prosjektet på en rad", "prosjektet med strek"],
    &["den valgte med bakgrunn", "den valgte med tykke streker", "den valgte med farget ramme"],
    &["ulest i oransje", "ulest i blått"],
    &["prosjektet uten sti", "stien etter prosjektet", "stien foran prosjektet", "stien under prosjektet"],
];
const PANER: [Rammer; 2] = [Rammer::Bokser, Rammer::Linjer];
const FARGER: [Farge; 2] = [Farge::Av, Farge::På];
const PLASSER: [Plass; 2] = [Plass::Vanlig, Plass::Minimal];
const ORDNER: [Orden; 3] = [Orden::Nerdr, Orden::Mappe, Orden::Aktivitet];
const LEDIGE: [Ledig; 2] = [Ledig::Under, Ledig::Midten];
const PROSJEKTER: [Prosjekter; 2] = [Prosjekter::Startet, Prosjekter::Alfabetisk];
const KORTENE: [Kort; 3] = [Kort::Bokser, Kort::Ramme, Kort::Kant];
const TOPPER: [Topp; 3] = [Topp::Tekst, Topp::Rad, Topp::Strek];
const VALGTE: [Valgt; 3] = [Valgt::Bakgrunn, Valgt::Tykk, Valgt::Ramme];
const ULESTFARGER: [Ulestfarge; 2] = [Ulestfarge::Oransje, Ulestfarge::Blå];
const STIER: [Sti; 4] = [Sti::Av, Sti::Etter, Sti::Før, Sti::Under];

impl Utseende {
    pub fn ny(paner: Rammer) -> Utseende {
        Utseende {
            paner,
            farge: Farge::På, plass: Plass::Vanlig, orden: Orden::Nerdr, ledig: Ledig::Under, tittel: 1, prosjekter: Prosjekter::Startet,
            kort: Kort::Bokser,
            topp: Topp::Tekst,
            valgt: Valgt::Bakgrunn,
            ulest: Ulestfarge::Oransje,
            sti: Sti::Av,
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
            6 => PROSJEKTER.iter().position(|p| *p == self.prosjekter),
            7 => KORTENE.iter().position(|k| *k == self.kort),
            8 => TOPPER.iter().position(|t| *t == self.topp),
            9 => VALGTE.iter().position(|v| *v == self.valgt),
            10 => ULESTFARGER.iter().position(|f| *f == self.ulest),
            _ => STIER.iter().position(|s| *s == self.sti),
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
            6 => self.prosjekter = PROSJEKTER[i],
            7 => self.kort = KORTENE[i],
            8 => self.topp = TOPPER[i],
            9 => self.valgt = VALGTE[i],
            10 => self.ulest = ULESTFARGER[i],
            _ => self.sti = STIER[i],
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
            MIDLERTIDIGE => self.orden == Orden::Mappe,
            KORT => self.plass != Plass::Minimal,
            TOPP => self.orden == Orden::Mappe && self.form() != Kort::Ramme,
            VALGT => self.plass != Plass::Minimal && self.form() != Kort::Kant,
            ULEST => self.farge == Farge::På,
            STI => self.topp != Topp::Tekst && self.orden == Orden::Mappe && self.form() != Kort::Ramme,
            _ => akse < BOKSAKSER || self.plass != Plass::Minimal,
        }
    }

    /// Kortene slik de tegnes: rammen trenger prosjektene og boksene.
    pub fn form(&self) -> Kort {
        match (self.plass, self.kort) {
            (Plass::Minimal, _) => Kort::Bokser,
            (_, Kort::Ramme) if self.orden != Orden::Mappe => Kort::Bokser,
            (_, kort) => kort,
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
        assert_eq!(u.tekst(), "paner=bokser farge=av plass=minimal orden=aktivitet ledig=under tittel=1 mapper=startet kort=bokser topp=tekst valgt=bakgrunn ulest=oransje sti=av");
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
