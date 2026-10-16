//! Rekkefølgen i lista: de som venter på deg først, så de som jobber, så de
//! du har sett.

use nerdr_kjerne::paner::Pane;
use nerdr_kjerne::register::{Sesjon, Tilstand};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Merke {
    Spør,
    Ferdig,
    Jobber,
    Sett,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Gruppe {
    VenterPåDeg,
    Jobber,
    Sett,
}

#[derive(Debug, PartialEq)]
pub enum Rad<'a> {
    Tom,
    Strek,
    Overskrift(Gruppe),
    Pane(&'a Pane, Merke),
}

impl Merke {
    /// `sett` er da du sist var inne i panen, i millisekunder siden 1970.
    pub fn fra(sesjon: &Sesjon, sett: u64) -> Merke {
        match sesjon.tilstand {
            Tilstand::Venter => Merke::Spør,
            Tilstand::Jobber => Merke::Jobber,
            Tilstand::Klar if sesjon.endret > sett => Merke::Ferdig,
            Tilstand::Klar => Merke::Sett,
        }
    }

    pub fn gruppe(self) -> Gruppe {
        match self {
            Merke::Spør | Merke::Ferdig => Gruppe::VenterPåDeg,
            Merke::Jobber => Gruppe::Jobber,
            Merke::Sett => Gruppe::Sett,
        }
    }
}

impl Gruppe {
    pub fn tittel(self) -> &'static str {
        match self {
            Gruppe::VenterPåDeg => "VENTER PÅ DEG",
            Gruppe::Jobber => "JOBBER",
            Gruppe::Sett => "SETT",
        }
    }
}

/// Panene gruppert, med en overskrift for hver gruppe som ikke er tom. Innenfor
/// en gruppe beholdes rekkefølgen fra tmux, så lista ikke stokkes om når bare
/// tidene endres.
pub fn rader<'a>(paner: &'a [Pane], merke: impl Fn(&Pane) -> Option<Merke>) -> Vec<Rad<'a>> {
    let mut merket: Vec<(&Pane, Merke)> = paner.iter().filter_map(|p| Some((p, merke(p)?))).collect();
    merket.sort_by_key(|(_, m)| m.gruppe());

    let mut rader = Vec::new();
    let mut forrige = None;
    for (pane, m) in merket {
        if forrige == Some(m.gruppe()) {
            rader.push(Rad::Strek);
        } else {
            if forrige.is_some() {
                rader.push(Rad::Tom);
            }
            rader.push(Rad::Overskrift(m.gruppe()));
            forrige = Some(m.gruppe());
        }
        rader.push(Rad::Pane(pane, m));
    }
    rader
}

#[cfg(test)]
mod tester {
    use super::*;

    fn sesjon(tilstand: Tilstand, endret: u64) -> Sesjon {
        Sesjon { tilstand, endret, mappe: None }
    }

    #[test]
    fn ferdig_etter_at_du_var_inne_venter_på_deg() {
        assert_eq!(Merke::fra(&sesjon(Tilstand::Klar, 20), 10), Merke::Ferdig);
        assert_eq!(Merke::fra(&sesjon(Tilstand::Klar, 10), 20), Merke::Sett);
        assert_eq!(Merke::fra(&sesjon(Tilstand::Venter, 10), 20), Merke::Spør);
    }

    #[test]
    fn grupperer_med_overskrifter_og_strek_mellom_panene() {
        let pane = |id: &str| Pane::fra_linje(&format!("$0\ts\t@0\t0\t{id}\tt\t80\t20\t80\t20")).unwrap();
        let paner = [pane("%1"), pane("%2"), pane("%3")];
        let rader = rader(&paner, |p| match p.id.as_str() {
            "%1" => Some(Merke::Sett),
            "%2" => Some(Merke::Ferdig),
            _ => Some(Merke::Spør),
        });
        assert_eq!(
            rader,
            [
                Rad::Overskrift(Gruppe::VenterPåDeg),
                Rad::Pane(&paner[1], Merke::Ferdig),
                Rad::Strek,
                Rad::Pane(&paner[2], Merke::Spør),
                Rad::Tom,
                Rad::Overskrift(Gruppe::Sett),
                Rad::Pane(&paner[0], Merke::Sett),
            ]
        );
    }
}
