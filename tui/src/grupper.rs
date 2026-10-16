//! Rekkefølgen i lista: de som venter på deg først, så de som jobber, så de
//! som er ferdige og sett.

use nerdr_kjerne::paner::Pane;
use nerdr_kjerne::register::{Sesjon, Tilstand};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Merke {
    Spør,
    /// Ferdig med noe du ikke har sett.
    Ulest,
    Jobber,
    Sett,
    /// Startet av nerdr og ikke i registeret ennå.
    Starter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Gruppe {
    Venter,
    Jobber,
    Ferdig,
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
            Tilstand::Klar if sesjon.endret > sett => Merke::Ulest,
            Tilstand::Klar => Merke::Sett,
        }
    }

    pub fn glyf(self) -> &'static str {
        match self {
            Merke::Spør => "●",
            Merke::Ulest => "◆",
            Merke::Jobber => "◐",
            Merke::Sett => "○",
            Merke::Starter => "◌",
        }
    }

    pub fn gruppe(self) -> Gruppe {
        match self {
            Merke::Spør | Merke::Ulest => Gruppe::Venter,
            Merke::Jobber | Merke::Starter => Gruppe::Jobber,
            Merke::Sett => Gruppe::Ferdig,
        }
    }
}

impl Gruppe {
    pub fn tittel(self) -> &'static str {
        match self {
            Gruppe::Venter => "VENTER",
            Gruppe::Jobber => "JOBBER",
            Gruppe::Ferdig => "FERDIG",
        }
    }
}

/// Panene gruppert, med en overskrift for hver gruppe som ikke er tom. Innenfor
/// en gruppe beholdes rekkefølgen fra tmux, så lista ikke stokkes om når bare
/// tidene endres. Gruppen gis for seg, fordi panen du er inne i, blir stående
/// der den var selv om merket endres.
pub fn rader<'a>(paner: &'a [Pane], plass: impl Fn(&Pane) -> Option<(Gruppe, Merke)>) -> Vec<Rad<'a>> {
    let mut plassert: Vec<(&Pane, Gruppe, Merke)> = paner
        .iter()
        .filter_map(|p| {
            let (g, m) = plass(p)?;
            Some((p, g, m))
        })
        .collect();
    plassert.sort_by_key(|(_, g, _)| *g);

    let mut rader = Vec::new();
    let mut forrige = None;
    for (pane, g, m) in plassert {
        if forrige == Some(g) {
            rader.push(Rad::Strek);
        } else {
            if forrige.is_some() {
                rader.push(Rad::Tom);
            }
            rader.push(Rad::Overskrift(g));
            forrige = Some(g);
        }
        rader.push(Rad::Pane(pane, m));
    }
    rader
}

#[cfg(test)]
mod tester {
    use super::*;

    fn sesjon(tilstand: Tilstand, endret: u64) -> Sesjon {
        Sesjon { tilstand, endret, mappe: None, samtale: None }
    }

    #[test]
    fn ferdig_etter_at_du_var_inne_venter_på_deg() {
        assert_eq!(Merke::fra(&sesjon(Tilstand::Klar, 20), 10), Merke::Ulest);
        assert_eq!(Merke::fra(&sesjon(Tilstand::Klar, 10), 20), Merke::Sett);
        assert_eq!(Merke::fra(&sesjon(Tilstand::Venter, 10), 20), Merke::Spør);
    }

    #[test]
    fn grupperer_med_overskrifter_og_strek_mellom_panene() {
        let pane = |id: &str| Pane::fra_linje(&format!("$0\ts\t@0\t0\t{id}\tt\t80\t20\t80\t20\t0\t0\t1\t\t1\t")).unwrap();
        let paner = [pane("%1"), pane("%2"), pane("%3")];
        let rader = rader(&paner, |p| {
            let m = match p.id.as_str() {
                "%1" => Merke::Sett,
                "%2" => Merke::Ulest,
                _ => Merke::Spør,
            };
            Some((m.gruppe(), m))
        });
        assert_eq!(
            rader,
            [
                Rad::Overskrift(Gruppe::Venter),
                Rad::Pane(&paner[1], Merke::Ulest),
                Rad::Strek,
                Rad::Pane(&paner[2], Merke::Spør),
                Rad::Tom,
                Rad::Overskrift(Gruppe::Ferdig),
                Rad::Pane(&paner[0], Merke::Sett),
            ]
        );
    }

    #[test]
    fn gruppen_kan_avvike_fra_merket() {
        let paner = [Pane::fra_linje("$0\ts\t@0\t0\t%1\tt\t80\t20\t80\t20\t0\t0\t1\t\t1\t").unwrap()];
        let rader = rader(&paner, |_| Some((Gruppe::Jobber, Merke::Sett)));
        assert_eq!(rader, [Rad::Overskrift(Gruppe::Jobber), Rad::Pane(&paner[0], Merke::Sett)]);
    }
}
