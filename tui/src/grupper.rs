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
    /// Startet av nerdr og ikke i registeret ennå.
    Starter,
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
    /// Gruppen og hvor mange som står i den.
    Overskrift(Gruppe, usize),
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

    pub fn glyf(self) -> &'static str {
        match self {
            Merke::Spør => "●",
            Merke::Ferdig => "◆",
            Merke::Jobber => "◐",
            Merke::Sett => "○",
            Merke::Starter => "◌",
        }
    }

    pub fn gruppe(self) -> Gruppe {
        match self {
            Merke::Spør | Merke::Ferdig => Gruppe::VenterPåDeg,
            Merke::Jobber | Merke::Starter => Gruppe::Jobber,
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

/// Panene gruppert, med en overskrift for hver gruppe, også de tomme, så
/// overskriftene ikke kommer og går. Innenfor en gruppe beholdes rekkefølgen
/// fra tmux, så lista ikke stokkes om når bare tidene endres. Gruppen gis for
/// seg, fordi panen du er inne i, blir stående der den var selv om merket
/// endres.
pub fn rader<'a>(paner: &'a [Pane], plass: impl Fn(&Pane) -> Option<(Gruppe, Merke)>) -> Vec<Rad<'a>> {
    let plassert: Vec<(&Pane, Gruppe, Merke)> = paner
        .iter()
        .filter_map(|p| {
            let (g, m) = plass(p)?;
            Some((p, g, m))
        })
        .collect();

    let mut rader = Vec::new();
    for gruppe in [Gruppe::VenterPåDeg, Gruppe::Jobber, Gruppe::Sett] {
        if !rader.is_empty() {
            rader.push(Rad::Tom);
        }
        let i_gruppen: Vec<_> = plassert.iter().filter(|(_, g, _)| *g == gruppe).collect();
        rader.push(Rad::Overskrift(gruppe, i_gruppen.len()));
        for (i, (pane, _, m)) in i_gruppen.into_iter().enumerate() {
            if i > 0 {
                rader.push(Rad::Strek);
            }
            rader.push(Rad::Pane(pane, *m));
        }
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
        assert_eq!(Merke::fra(&sesjon(Tilstand::Klar, 20), 10), Merke::Ferdig);
        assert_eq!(Merke::fra(&sesjon(Tilstand::Klar, 10), 20), Merke::Sett);
        assert_eq!(Merke::fra(&sesjon(Tilstand::Venter, 10), 20), Merke::Spør);
    }

    #[test]
    fn alle_gruppene_har_overskrift_og_strek_mellom_panene() {
        let pane = |id: &str| Pane::fra_linje(&format!("$0\ts\t@0\t0\t{id}\tt\t80\t20\t80\t20\t0\t0\t1\t\t1\t")).unwrap();
        let paner = [pane("%1"), pane("%2"), pane("%3")];
        let rader = rader(&paner, |p| {
            let m = match p.id.as_str() {
                "%1" => Merke::Sett,
                "%2" => Merke::Ferdig,
                _ => Merke::Spør,
            };
            Some((m.gruppe(), m))
        });
        assert_eq!(
            rader,
            [
                Rad::Overskrift(Gruppe::VenterPåDeg, 2),
                Rad::Pane(&paner[1], Merke::Ferdig),
                Rad::Strek,
                Rad::Pane(&paner[2], Merke::Spør),
                Rad::Tom,
                Rad::Overskrift(Gruppe::Jobber, 0),
                Rad::Tom,
                Rad::Overskrift(Gruppe::Sett, 1),
                Rad::Pane(&paner[0], Merke::Sett),
            ]
        );
    }

    #[test]
    fn gruppen_kan_avvike_fra_merket() {
        let paner = [Pane::fra_linje("$0\ts\t@0\t0\t%1\tt\t80\t20\t80\t20\t0\t0\t1\t\t1\t").unwrap()];
        let rader = rader(&paner, |_| Some((Gruppe::Jobber, Merke::Sett)));
        assert_eq!(rader[2..4], [Rad::Overskrift(Gruppe::Jobber, 1), Rad::Pane(&paner[0], Merke::Sett)]);
    }
}
