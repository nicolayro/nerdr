//! Rekkefølgen i lista: de som venter på deg først, så de som jobber, så de
//! du har sett. En terminal står under Claude-en den hører til.

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
    Overskrift(Gruppe),
    Pane(&'a Pane, Merke),
    Terminal(&'a Pane),
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

/// Panene gruppert, med en overskrift for hver gruppe som ikke er tom. Innenfor
/// en gruppe beholdes rekkefølgen fra tmux, så lista ikke stokkes om når bare
/// tidene endres. Gruppen gis for seg, fordi panen du er inne i, blir stående
/// der den var selv om merket endres.
///
/// En pane uten plass er en terminal når den er knyttet til en Claude. Kjører
/// det en Claude i en terminal, har den plass og får sitt eget kort.
pub fn rader<'a>(paner: &'a [Pane], plass: impl Fn(&Pane) -> Option<(Gruppe, Merke)>) -> Vec<Rad<'a>> {
    let mut plassert = Vec::new();
    let mut terminaler = Vec::new();
    for p in paner {
        match plass(p) {
            Some((g, m)) => plassert.push((p, g, m)),
            None if p.claude.is_some() => terminaler.push(p),
            None => {}
        }
    }
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
        for t in terminaler.iter().filter(|t| t.claude.as_ref() == Some(&pane.id)) {
            rader.push(Rad::Terminal(t));
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
    fn grupperer_med_overskrifter_og_strek_mellom_panene() {
        let pane = |id: &str| Pane::fra_linje(&format!("$0\ts\t@0\t0\t{id}\tt\t80\t20\t80\t20\tzsh\t")).unwrap();
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

    #[test]
    fn terminalen_står_under_claude_en_sin() {
        let pane = |id: &str, claude: &str| {
            Pane::fra_linje(&format!("$0\ts\t@0\t0\t{id}\tt\t80\t20\t80\t20\tzsh\t{claude}")).unwrap()
        };
        let paner = [pane("%3", "%2"), pane("%1", ""), pane("%2", ""), pane("%4", "%9")];
        let rader = rader(&paner, |p| match p.id.as_str() {
            "%1" => Some((Gruppe::VenterPåDeg, Merke::Spør)),
            "%2" => Some((Gruppe::Sett, Merke::Sett)),
            _ => None,
        });
        assert_eq!(
            rader,
            [
                Rad::Overskrift(Gruppe::VenterPåDeg),
                Rad::Pane(&paner[1], Merke::Spør),
                Rad::Tom,
                Rad::Overskrift(Gruppe::Sett),
                Rad::Pane(&paner[2], Merke::Sett),
                Rad::Terminal(&paner[0]),
            ]
        );
    }

    #[test]
    fn gruppen_kan_avvike_fra_merket() {
        let paner = [Pane::fra_linje("$0\ts\t@0\t0\t%1\tt\t80\t20\t80\t20\tzsh\t").unwrap()];
        let rader = rader(&paner, |_| Some((Gruppe::Jobber, Merke::Sett)));
        assert_eq!(rader, [Rad::Overskrift(Gruppe::Jobber), Rad::Pane(&paner[0], Merke::Sett)]);
    }
}
