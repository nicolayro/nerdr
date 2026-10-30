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

/// Fargen til det som er ferdig og ikke sett.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ulestfarge {
    /// Som det som spør.
    Oransje,
    Blå,
}

/// Hvordan prosjektet står over Claude-ene, når lista er sortert etter
/// prosjekt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Topp {
    /// En rad med bakgrunn.
    Rad,
    /// En strek fra navnet til skillelinja.
    Strek,
    /// Strek på begge sider av navnet.
    Venstre,
}

/// Mappa Claude-en jobber i, under branchen, når den ikke er mappa til repoet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sti {
    Av,
    På,
}

/// Bredden på agentlista. Du kan også dra den til en hvilken som helst
/// bredde, og da står nivået som er nærmest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bredde {
    Smal,
    Vanlig,
    Bred,
}

impl Bredde {
    pub fn kolonner(self) -> u16 {
        match self {
            Bredde::Smal => 30,
            Bredde::Vanlig => 36,
            Bredde::Bred => 48,
        }
    }

    pub fn nærmest(kolonner: u16) -> Bredde {
        BREDDER.into_iter().min_by_key(|b| b.kolonner().abs_diff(kolonner)).unwrap_or(Bredde::Vanlig)
    }
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
    pub ulest: Ulestfarge,
    pub topp: Topp,
    pub sti: Sti,
    pub bredde: Bredde,
}

/// Står i `OPTION` foran verdien, som `farge=på`, så en akse kan komme til
/// eller forsvinne uten at de andre leses feil.
pub const AKSER: [&str; 10] = ["paner", "farge", "plass", "orden", "ledig", "tittel", "ulest", "topp", "sti", "bredde"];
pub const ORDEN: usize = 3;
const LEDIG: usize = 4;
const ULEST: usize = 6;
const TOPP: usize = 7;
pub const BREDDE: usize = 9;
/// Aksene fra denne gjelder bare boksene.
pub const BOKSAKSER: usize = 5;
/// Står i `OPTION`, og endres ikke.
const VERDIER: [&[&str]; 10] = [
    &["bokser", "linjer"],
    &["av", "på"],
    &["vanlig", "minimal"],
    &["nerdr", "mappe", "aktivitet"],
    &["under", "midten"],
    &["1", "2", "3"],
    &["oransje", "blå"],
    &["rad", "strek", "venstre"],
    &["av", "på"],
    &["smal", "vanlig", "bred"],
];
/// Det sorteringen heter i menyen og på knappen, som `VERDIER` for orden.
pub const SORTERINGER: [&str; 3] = ["prioritert", "prosjekt", "aktivitet"];
/// Navnene i menyen, i samme rekkefølge som `AKSER`.
const I_MENYEN: [&str; 10] =
    ["ramme", "farge", "kortdesign", "sortering", "ledige", "oppgave", "ulest", "overskrift", "mappe", "bredde"];
/// Rekkefølgen i menyen, under overskriften. Et valg som bare gjelder for
/// valget over, står rett under det.
pub const MENY: [(&str, &[usize]); 2] = [("Pane", &[0]), ("Agentliste", &[9, 1, 6, 2, 5, 8, 3, 7, 4])];
/// Står nederst i menyen for valget du står på, i samme rekkefølge som `AKSER`.
pub const FORKLARING: [&str; 10] = [
    "Bokser gir hver pane en ramme, og naboer deler kanten. Linjer lar panene fylle visningen, med streker mellom.",
    "Tegnet og tiden får fargen til gruppen, og rammene en tone av den.",
    "Vanlig har en boks for hver sesjon. Minimal har én linje for hver.",
    "Prioritert har de aktive øverst. Prosjekt grupperer etter repo. Aktivitet har den som sist endret seg, øverst.",
    "Om de ledige står rett under de aktive, eller fra midten, så de ikke flytter seg.",
    "Hvor mange linjer oppgaven får i kortet.",
    "Fargen til det som er ferdig og ikke sett. Oransje er som det som spør, og blå skiller dem.",
    "Prosjektet over sesjonene, på en rad med bakgrunn, med en strek bort til skillelinja, eller med strek på begge sider.",
    "Mappa under branchen når Claude-en ikke jobber i mappa til repoet: stien til et worktree, eller en undermappe i repoet.",
    "Hvor bred agentlista er. Du kan også dra i streken til høyre for den.",
];
/// Verdiene som i handlingene, i samme rekkefølge som `VERDIER`.
pub const NAVN: [&[&str]; 10] = [
    &["paner i bokser", "paner med linjer"],
    &["uten farge", "med farge"],
    &["vanlig liste", "minimal liste"],
    &["sorter prioritert", "sorter etter prosjekt", "sorter etter aktivitet"],
    &["ledige under de aktive", "ledige fra midten"],
    &["oppgaven på 1 linje", "oppgaven på 2 linjer", "oppgaven på 3 linjer"],
    &["ulest i oransje", "ulest i blått"],
    &["prosjektet på en rad", "prosjektet med strek", "prosjektet med strek til venstre"],
    &["uten mappe i kortet", "med mappe i kortet"],
    &["smal agentliste", "vanlig agentliste", "bred agentliste"],
];
const PANER: [Rammer; 2] = [Rammer::Bokser, Rammer::Linjer];
const FARGER: [Farge; 2] = [Farge::Av, Farge::På];
const PLASSER: [Plass; 2] = [Plass::Vanlig, Plass::Minimal];
const ORDNER: [Orden; 3] = [Orden::Nerdr, Orden::Mappe, Orden::Aktivitet];
const LEDIGE: [Ledig; 2] = [Ledig::Under, Ledig::Midten];
const ULESTFARGER: [Ulestfarge; 2] = [Ulestfarge::Oransje, Ulestfarge::Blå];
const TOPPER: [Topp; 3] = [Topp::Rad, Topp::Strek, Topp::Venstre];
const STIER: [Sti; 2] = [Sti::Av, Sti::På];
const BREDDER: [Bredde; 3] = [Bredde::Smal, Bredde::Vanlig, Bredde::Bred];

impl Utseende {
    pub fn ny(paner: Rammer) -> Utseende {
        Utseende {
            paner,
            farge: Farge::På,
            plass: Plass::Vanlig,
            orden: Orden::Nerdr,
            ledig: Ledig::Under,
            tittel: 1,
            ulest: Ulestfarge::Oransje,
            topp: Topp::Venstre,
            sti: Sti::På,
            bredde: Bredde::Vanlig,
        }
    }

    /// Navnet på aksen og verdiene i menyen.
    pub fn i_menyen(akse: usize) -> (&'static str, &'static [&'static str]) {
        (I_MENYEN[akse], if akse == ORDEN { &SORTERINGER } else { VERDIER[akse] })
    }

    /// Valgene som vises i menyen, i rekkefølgen der.
    pub fn synlige(&self) -> Vec<usize> {
        MENY.iter().flat_map(|(_, akser)| akser.iter().copied()).filter(|&a| self.gjelder(a)).collect()
    }

    /// Valget over eller under `akse` i menyen.
    pub fn nabo(&self, akse: usize, steg: isize) -> usize {
        let synlige = self.synlige();
        let i = synlige.iter().position(|&a| a == akse).unwrap_or(0) as isize;
        synlige[(i + steg).clamp(0, synlige.len() as isize - 1) as usize]
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
            6 => ULESTFARGER.iter().position(|f| *f == self.ulest),
            7 => TOPPER.iter().position(|t| *t == self.topp),
            8 => STIER.iter().position(|s| *s == self.sti),
            _ => BREDDER.iter().position(|b| *b == self.bredde),
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
            6 => self.ulest = ULESTFARGER[i],
            7 => self.topp = TOPPER[i],
            8 => self.sti = STIER[i],
            _ => self.bredde = BREDDER[i],
        }
    }

    /// Neste eller forrige verdi på aksen, rundt.
    pub fn endre(&mut self, akse: usize, steg: i32) {
        let antall = VERDIER[akse].len() as i32;
        self.sett(akse, (self.indeks(akse) as i32 + steg).rem_euclid(antall) as usize);
    }

    /// Minimal har ingen bokser å velge for, bare prioritert har ledige, og
    /// bare prosjekt har overskrifter for prosjektene.
    pub fn gjelder(&self, akse: usize) -> bool {
        match akse {
            LEDIG => self.orden == Orden::Nerdr,
            ULEST => self.farge == Farge::På,
            TOPP => self.orden == Orden::Mappe,
            BREDDE => true,
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
        assert_eq!(u.tekst(), "paner=bokser farge=av plass=minimal orden=aktivitet ledig=under tittel=1 ulest=oransje topp=venstre sti=på bredde=vanlig");
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
    fn menyen_viser_bare_valg_som_gjelder() {
        let mut u = Utseende::ny(Rammer::Linjer);
        assert_eq!(u.synlige(), [0, 9, 1, 6, 2, 5, 8, 3, 4]);
        u.plass = Plass::Minimal;
        u.orden = Orden::Mappe;
        assert_eq!(u.synlige(), [0, 9, 1, 6, 2, 3, 7]);
        assert_eq!(u.nabo(2, 1), 3);
        assert_eq!(u.nabo(0, -1), 0);
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
