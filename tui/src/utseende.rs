//! Hvordan lista og panene ser ut, valgt i utseende-menyen og husket i tmux.

use crate::bokser::Rammer;

/// Global option på tmux-serveren, så valget gjelder alle som kjører nerdr
/// mot den, og overlever omstart.
pub const OPTION: &str = "@nerdr-utseende";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kortdesign {
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

/// Knappene nederst i agentlista, for dem som klikker framfor prefikset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Snarveier {
    Av,
    På,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Utseende {
    pub paner: Rammer,
    pub kortdesign: Kortdesign,
    pub orden: Orden,
    /// Linjer til tittelen i boksene.
    pub tittel: usize,
    pub bredde: Bredde,
    pub snarveier: Snarveier,
}

/// Et valg i innstillingene. `nøkkel` står i `OPTION` foran verdien, som
/// `plass=minimal`, så en akse kan komme til eller forsvinne uten at de andre
/// leses feil.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Akse {
    Paner,
    Kortdesign,
    Orden,
    Tittel,
    Bredde,
    Snarveier,
}

/// En verdi på en akse. `nøkkel` står i `OPTION` og endres ikke, `meny` er
/// teksten i innstillingene og `navn` teksten i handlingene.
pub struct Verdi {
    pub nøkkel: &'static str,
    pub meny: &'static str,
    pub navn: &'static str,
}

const fn v(nøkkel: &'static str, navn: &'static str) -> Verdi {
    Verdi { nøkkel, meny: nøkkel, navn }
}

/// Rekkefølgen i menyen, under overskriften. Et valg som bare gjelder for
/// valget over, står rett under det.
pub const MENY: [(&str, &[Akse]); 2] =
    [("Pane", &[Akse::Paner]), ("Agentliste", &[Akse::Bredde, Akse::Kortdesign, Akse::Tittel, Akse::Orden, Akse::Snarveier])];

impl Akse {
    /// Rekkefølgen i `OPTION`.
    pub const ALLE: [Akse; 6] = [Akse::Paner, Akse::Kortdesign, Akse::Orden, Akse::Tittel, Akse::Bredde, Akse::Snarveier];

    pub fn nøkkel(self) -> &'static str {
        match self {
            Akse::Paner => "paner",
            Akse::Kortdesign => "plass",
            Akse::Orden => "orden",
            Akse::Tittel => "tittel",
            Akse::Bredde => "bredde",
            Akse::Snarveier => "snarveier",
        }
    }

    /// Navnet i menyen.
    pub fn i_menyen(self) -> &'static str {
        match self {
            Akse::Paner => "ramme",
            Akse::Kortdesign => "kortdesign",
            Akse::Orden => "sortering",
            Akse::Tittel => "oppgave",
            Akse::Bredde => "bredde",
            Akse::Snarveier => "snarveier",
        }
    }

    /// Står nederst i menyen for valget du står på.
    pub fn forklaring(self) -> &'static str {
        match self {
            Akse::Paner => "Bokser rammer inn hver pane, og linjer har streker mellom. Vindu har også ramme rundt agentlista.",
            Akse::Kortdesign => "Vanlig har en boks for hver sesjon. Minimal har én linje for hver.",
            Akse::Orden => "Prioritert har de aktive øverst. Prosjekt grupperer etter repo. Aktivitet har den som sist endret seg, øverst.",
            Akse::Tittel => "Hvor mange linjer oppgaven får i kortet.",
            Akse::Bredde => "Hvor bred agentlista er. Du kan også dra i streken til høyre for den.",
            Akse::Snarveier => "Knapper nederst i agentlista for ny sesjon, hent samtale, ny terminal, innstillinger og handlinger.",
        }
    }

    /// I samme rekkefølge som typene i `indeks` og `sett`.
    pub fn verdier(self) -> &'static [Verdi] {
        match self {
            Akse::Paner => &PANER_VERDIER,
            Akse::Kortdesign => &KORTDESIGN_VERDIER,
            Akse::Orden => &ORDEN_VERDIER,
            Akse::Tittel => &TITTEL_VERDIER,
            Akse::Bredde => &BREDDE_VERDIER,
            Akse::Snarveier => &SNARVEIER_VERDIER,
        }
    }
}

const PANER_VERDIER: [Verdi; 3] = [v("bokser", "paner i bokser"), v("linjer", "paner med linjer"), v("vindu", "paner i et vindu")];
const KORTDESIGN_VERDIER: [Verdi; 2] = [v("vanlig", "vanlig liste"), v("minimal", "minimal liste")];
const ORDEN_VERDIER: [Verdi; 3] = [
    Verdi { nøkkel: "nerdr", meny: "prioritert", navn: "sorter prioritert" },
    Verdi { nøkkel: "mappe", meny: "prosjekt", navn: "sorter etter prosjekt" },
    Verdi { nøkkel: "aktivitet", meny: "aktivitet", navn: "sorter etter aktivitet" },
];
const TITTEL_VERDIER: [Verdi; 3] = [v("1", "oppgaven på 1 linje"), v("2", "oppgaven på 2 linjer"), v("3", "oppgaven på 3 linjer")];
const BREDDE_VERDIER: [Verdi; 3] = [v("smal", "smal agentliste"), v("vanlig", "vanlig agentliste"), v("bred", "bred agentliste")];
const SNARVEIER_VERDIER: [Verdi; 2] = [v("av", "uten snarveier nederst"), v("på", "med snarveier nederst")];

const PANER: [Rammer; 3] = [Rammer::Bokser, Rammer::Linjer, Rammer::Vindu];
const KORTDESIGN: [Kortdesign; 2] = [Kortdesign::Vanlig, Kortdesign::Minimal];
const ORDNER: [Orden; 3] = [Orden::Nerdr, Orden::Mappe, Orden::Aktivitet];
const BREDDER: [Bredde; 3] = [Bredde::Smal, Bredde::Vanlig, Bredde::Bred];
const SNARVEIENE: [Snarveier; 2] = [Snarveier::Av, Snarveier::På];

impl Utseende {
    pub fn ny(paner: Rammer) -> Utseende {
        Utseende {
            paner,
            kortdesign: Kortdesign::Vanlig,
            orden: Orden::Nerdr,
            tittel: 1,
            bredde: Bredde::Vanlig,
            snarveier: Snarveier::Av,
        }
    }

    /// Valgene som vises i menyen, i rekkefølgen der.
    pub fn synlige(&self) -> Vec<Akse> {
        MENY.iter().flat_map(|(_, akser)| akser.iter().copied()).filter(|&a| self.gjelder(a)).collect()
    }

    /// Valget over eller under `akse` i menyen.
    pub fn nabo(&self, akse: Akse, steg: isize) -> Akse {
        let synlige = self.synlige();
        let i = synlige.iter().position(|&a| a == akse).unwrap_or(0) as isize;
        synlige[(i + steg).clamp(0, synlige.len() as isize - 1) as usize]
    }

    /// Verdien som er valgt på aksen, som indeks i `verdier`.
    pub fn indeks(&self, akse: Akse) -> usize {
        let i = match akse {
            Akse::Paner => PANER.iter().position(|p| *p == self.paner),
            Akse::Kortdesign => KORTDESIGN.iter().position(|p| *p == self.kortdesign),
            Akse::Orden => ORDNER.iter().position(|o| *o == self.orden),
            Akse::Tittel => Some(self.tittel - 1),
            Akse::Bredde => BREDDER.iter().position(|b| *b == self.bredde),
            Akse::Snarveier => SNARVEIENE.iter().position(|k| *k == self.snarveier),
        };
        i.unwrap_or(0)
    }

    pub fn sett(&mut self, akse: Akse, i: usize) {
        match akse {
            Akse::Paner => self.paner = PANER[i],
            Akse::Kortdesign => self.kortdesign = KORTDESIGN[i],
            Akse::Orden => self.orden = ORDNER[i],
            Akse::Tittel => self.tittel = i + 1,
            Akse::Bredde => self.bredde = BREDDER[i],
            Akse::Snarveier => self.snarveier = SNARVEIENE[i],
        }
    }

    /// Neste eller forrige verdi på aksen, rundt.
    pub fn endre(&mut self, akse: Akse, steg: i32) {
        let antall = akse.verdier().len() as i32;
        self.sett(akse, (self.indeks(akse) as i32 + steg).rem_euclid(antall) as usize);
    }

    /// Minimal har ingen bokser å velge oppgavelinjer for.
    pub fn gjelder(&self, akse: Akse) -> bool {
        match akse {
            Akse::Tittel => self.kortdesign != Kortdesign::Minimal,
            Akse::Paner | Akse::Kortdesign | Akse::Orden | Akse::Bredde | Akse::Snarveier => true,
        }
    }

    pub fn tekst(&self) -> String {
        Akse::ALLE.iter().map(|a| format!("{}={}", a.nøkkel(), a.verdier()[self.indeks(*a)].nøkkel)).collect::<Vec<_>>().join(" ")
    }

    /// Akser og verdier nerdr ikke kjenner, endrer ingenting. Det gjelder også
    /// de som er fjernet, så en gammel `OPTION` leses uten feil.
    pub fn les(&mut self, tekst: &str) {
        for (navn, verdi) in tekst.split_whitespace().filter_map(|o| o.split_once('=')) {
            if let Some(akse) = Akse::ALLE.into_iter().find(|a| a.nøkkel() == navn)
                && let Some(i) = akse.verdier().iter().position(|v| v.nøkkel == verdi)
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
        u.endre(Akse::Kortdesign, 1);
        u.endre(Akse::Orden, -1);
        assert_eq!(u.tekst(), "paner=bokser plass=minimal orden=aktivitet tittel=1 bredde=vanlig snarveier=av");
        let mut lest = Utseende::ny(Rammer::Linjer);
        lest.les(&u.tekst());
        assert_eq!(lest, u);
    }

    #[test]
    fn ukjente_akser_og_verdier_endrer_ingenting() {
        let mut u = Utseende::ny(Rammer::Linjer);
        u.les("lista=original paner=sirkler bokser plass=minimal");
        assert_eq!((u.paner, u.kortdesign), (Rammer::Linjer, Kortdesign::Minimal));
    }

    #[test]
    fn fjernede_akser_i_en_gammel_option_leses_uten_feil() {
        let mut u = Utseende::ny(Rammer::Linjer);
        u.les("paner=bokser farge=av plass=minimal orden=aktivitet ledig=midten tittel=2 ulest=blå topp=rad sti=av bredde=bred snarveier=på");
        assert_eq!(
            u,
            Utseende { paner: Rammer::Bokser, kortdesign: Kortdesign::Minimal, orden: Orden::Aktivitet, tittel: 2, bredde: Bredde::Bred, snarveier: Snarveier::På }
        );
    }

    #[test]
    fn hver_akse_har_en_verdi_for_hver_indeks() {
        for akse in Akse::ALLE {
            let mut u = Utseende::ny(Rammer::Linjer);
            for i in 0..akse.verdier().len() {
                u.sett(akse, i);
                assert_eq!(u.indeks(akse), i, "{}", akse.nøkkel());
            }
        }
    }

    #[test]
    fn menyen_viser_bare_valg_som_gjelder() {
        let mut u = Utseende::ny(Rammer::Linjer);
        assert_eq!(u.synlige(), [Akse::Paner, Akse::Bredde, Akse::Kortdesign, Akse::Tittel, Akse::Orden, Akse::Snarveier]);
        u.kortdesign = Kortdesign::Minimal;
        assert_eq!(u.synlige(), [Akse::Paner, Akse::Bredde, Akse::Kortdesign, Akse::Orden, Akse::Snarveier]);
        assert_eq!(u.nabo(Akse::Kortdesign, 1), Akse::Orden);
        assert_eq!(u.nabo(Akse::Paner, -1), Akse::Paner);
    }

    #[test]
    fn endre_går_rundt() {
        let mut u = Utseende::ny(Rammer::Linjer);
        u.endre(Akse::Paner, 1);
        assert_eq!(u.paner, Rammer::Vindu);
        u.endre(Akse::Paner, 1);
        assert_eq!(u.paner, Rammer::Bokser);
        u.endre(Akse::Orden, 4);
        assert_eq!(u.orden, Orden::Mappe);
    }
}
