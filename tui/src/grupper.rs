//! Rekkefølgen i lista: de som venter på deg først, så resten.

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
    Resten,
}

#[derive(Debug, PartialEq)]
pub enum Rad<'a> {
    Tom,
    Overskrift(Gruppe),
    /// Med mappe, branch og oppgave.
    Kort(&'a Pane, Merke),
    /// Med mappe og branch.
    Linje(&'a Pane, Merke),
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
            Merke::Jobber | Merke::Starter | Merke::Sett => Gruppe::Resten,
        }
    }
}

impl Gruppe {
    pub fn tittel(self) -> &'static str {
        match self {
            Gruppe::VenterPåDeg => "VENTER PÅ DEG",
            Gruppe::Resten => "RESTEN",
        }
    }
}

/// Det som venter på deg, som kort med oppgaven, og under det resten, én
/// linje hver. Innenfor en gruppe beholdes rekkefølgen fra tmux, så lista ikke
/// stokkes om når bare tidene endres. Gruppen gis for seg, fordi panen du er
/// inne i, blir stående der den var selv om merket endres.
pub fn rader<'a>(paner: &'a [Pane], plass: impl Fn(&Pane) -> Option<(Gruppe, Merke)>) -> Vec<Rad<'a>> {
    let plassert: Vec<(&Pane, Gruppe, Merke)> = paner
        .iter()
        .filter_map(|p| {
            let (g, m) = plass(p)?;
            Some((p, g, m))
        })
        .collect();
    let i = |gruppe| plassert.iter().filter(move |(_, g, _)| *g == gruppe).map(|(p, _, m)| (*p, *m));

    let mut rader = vec![Rad::Overskrift(Gruppe::VenterPåDeg)];
    rader.extend(i(Gruppe::VenterPåDeg).map(|(p, m)| Rad::Kort(p, m)));
    rader.push(Rad::Tom);
    rader.push(Rad::Overskrift(Gruppe::Resten));
    rader.extend(i(Gruppe::Resten).map(|(p, m)| Rad::Linje(p, m)));
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
    fn det_som_venter_som_kort_og_resten_som_linjer_i_rekkefølgen_fra_tmux() {
        let pane = |id: &str| Pane::fra_linje(&format!("$0\ts\t@0\t0\t{id}\tt\t80\t20\t80\t20\t0\t0\t1\t\t1\t")).unwrap();
        let paner = [pane("%1"), pane("%2"), pane("%3"), pane("%4")];
        let rader = rader(&paner, |p| {
            let m = match p.id.as_str() {
                "%1" => Merke::Sett,
                "%2" => Merke::Ferdig,
                "%3" => Merke::Jobber,
                _ => Merke::Spør,
            };
            Some((m.gruppe(), m))
        });
        assert_eq!(
            rader,
            [
                Rad::Overskrift(Gruppe::VenterPåDeg),
                Rad::Kort(&paner[1], Merke::Ferdig),
                Rad::Kort(&paner[3], Merke::Spør),
                Rad::Tom,
                Rad::Overskrift(Gruppe::Resten),
                Rad::Linje(&paner[0], Merke::Sett),
                Rad::Linje(&paner[2], Merke::Jobber),
            ]
        );
    }

    #[test]
    fn gruppen_kan_avvike_fra_merket() {
        let paner = [Pane::fra_linje("$0\ts\t@0\t0\t%1\tt\t80\t20\t80\t20\t0\t0\t1\t\t1\t").unwrap()];
        let rader = rader(&paner, |_| Some((Gruppe::VenterPåDeg, Merke::Sett)));
        assert_eq!(rader[1], Rad::Kort(&paner[0], Merke::Sett));
    }
}
