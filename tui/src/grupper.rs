//! Rekkefølgen i lista. Mens du er i panen, står den du er i, øverst, så det
//! som trenger deg, og bare antallet av resten. I lista står alle, gruppert.

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
    /// Den du er inne i, øverst mens du er i panen.
    Her,
    Spør,
    Ferdig,
    Jobber,
    Sett,
}

#[derive(Debug, PartialEq)]
pub enum Rad<'a> {
    Tom,
    Overskrift(Gruppe),
    /// Med mappe, branch og oppgave.
    Kort(&'a Pane, Merke),
    /// Med mappe og branch.
    Linje(&'a Pane, Merke),
    /// Hvor mange som jobber og er sett, mens bare det som trenger deg vises.
    Antall { jobber: usize, sett: usize },
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
            Merke::Spør => Gruppe::Spør,
            Merke::Ferdig => Gruppe::Ferdig,
            Merke::Jobber | Merke::Starter => Gruppe::Jobber,
            Merke::Sett => Gruppe::Sett,
        }
    }
}

impl Gruppe {
    pub fn tittel(self) -> &'static str {
        match self {
            Gruppe::Her => "HER",
            Gruppe::Spør => "SPØR",
            Gruppe::Ferdig => "FERDIG",
            Gruppe::Jobber => "JOBBER",
            Gruppe::Sett => "SETT",
        }
    }
}

/// Radene i lista. `her` er den du er inne i, og med `alle` står alle i
/// gruppene sine, som når du er i lista. Uten står `her` øverst, så det som
/// spør eller er ferdig, og bare antallet av resten.
///
/// En gruppe uten noen i seg har ingen overskrift. Innenfor en gruppe beholdes
/// rekkefølgen fra tmux, så lista ikke stokkes om når bare tidene endres.
/// Gruppen gis for seg, fordi panen du er inne i, blir stående der den var
/// selv om merket endres.
pub fn rader<'a>(
    paner: &'a [Pane],
    plass: impl Fn(&Pane) -> Option<(Gruppe, Merke)>,
    her: Option<&str>,
    alle: bool,
) -> Vec<Rad<'a>> {
    let plassert: Vec<(&Pane, Gruppe, Merke)> = paner
        .iter()
        .filter_map(|p| {
            let (g, m) = plass(p)?;
            Some((p, g, m))
        })
        .collect();
    let er_her = |p: &Pane| !alle && Some(p.id.as_str()) == her;
    let i = |gruppe| plassert.iter().filter(move |(p, g, _)| *g == gruppe && !er_her(p)).map(|(p, _, m)| (*p, *m));

    let mut rader = Vec::new();
    let gruppe = |rader: &mut Vec<Rad<'a>>, gruppe: Gruppe, kort: bool| {
        let i_gruppen: Vec<_> = i(gruppe).collect();
        if i_gruppen.is_empty() {
            return;
        }
        if !rader.is_empty() {
            rader.push(Rad::Tom);
        }
        rader.push(Rad::Overskrift(gruppe));
        rader.extend(i_gruppen.into_iter().map(|(p, m)| if kort { Rad::Kort(p, m) } else { Rad::Linje(p, m) }));
    };
    if !alle && let Some((p, _, m)) = plassert.iter().find(|(p, _, _)| er_her(p)) {
        rader.push(Rad::Overskrift(Gruppe::Her));
        rader.push(Rad::Kort(p, *m));
    }
    gruppe(&mut rader, Gruppe::Spør, true);
    gruppe(&mut rader, Gruppe::Ferdig, true);
    if alle {
        gruppe(&mut rader, Gruppe::Jobber, false);
        gruppe(&mut rader, Gruppe::Sett, false);
    } else {
        let (jobber, sett) = (i(Gruppe::Jobber).count(), i(Gruppe::Sett).count());
        if jobber + sett > 0 {
            rader.push(Rad::Tom);
            rader.push(Rad::Antall { jobber, sett });
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
    fn ferdig_til_du_har_vært_inne() {
        assert_eq!(Merke::fra(&sesjon(Tilstand::Klar, 20), 10), Merke::Ferdig);
        assert_eq!(Merke::fra(&sesjon(Tilstand::Klar, 10), 20), Merke::Sett);
        assert_eq!(Merke::fra(&sesjon(Tilstand::Venter, 10), 20), Merke::Spør);
    }

    fn paner() -> [Pane; 5] {
        let pane = |id: &str| Pane::fra_linje(&format!("$0\ts\t@0\t0\t{id}\tt\t80\t20\t80\t20\t0\t0\t1\t\t1\t")).unwrap();
        [pane("%1"), pane("%2"), pane("%3"), pane("%4"), pane("%5")]
    }

    fn plass(p: &Pane) -> Option<(Gruppe, Merke)> {
        let m = match p.id.as_str() {
            "%1" => Merke::Sett,
            "%2" => Merke::Ferdig,
            "%3" | "%5" => Merke::Jobber,
            _ => Merke::Spør,
        };
        Some((m.gruppe(), m))
    }

    #[test]
    fn i_lista_står_alle_i_gruppene_sine() {
        let paner = paner();
        assert_eq!(
            rader(&paner, plass, Some("%3"), true),
            [
                Rad::Overskrift(Gruppe::Spør),
                Rad::Kort(&paner[3], Merke::Spør),
                Rad::Tom,
                Rad::Overskrift(Gruppe::Ferdig),
                Rad::Kort(&paner[1], Merke::Ferdig),
                Rad::Tom,
                Rad::Overskrift(Gruppe::Jobber),
                Rad::Linje(&paner[2], Merke::Jobber),
                Rad::Linje(&paner[4], Merke::Jobber),
                Rad::Tom,
                Rad::Overskrift(Gruppe::Sett),
                Rad::Linje(&paner[0], Merke::Sett),
            ]
        );
    }

    #[test]
    fn i_panen_står_den_du_er_i_øverst_og_bare_antallet_av_resten() {
        let paner = paner();
        assert_eq!(
            rader(&paner, plass, Some("%3"), false),
            [
                Rad::Overskrift(Gruppe::Her),
                Rad::Kort(&paner[2], Merke::Jobber),
                Rad::Tom,
                Rad::Overskrift(Gruppe::Spør),
                Rad::Kort(&paner[3], Merke::Spør),
                Rad::Tom,
                Rad::Overskrift(Gruppe::Ferdig),
                Rad::Kort(&paner[1], Merke::Ferdig),
                Rad::Tom,
                Rad::Antall { jobber: 1, sett: 1 },
            ]
        );
    }

    #[test]
    fn gruppen_kan_avvike_fra_merket() {
        let paner = [Pane::fra_linje("$0\ts\t@0\t0\t%1\tt\t80\t20\t80\t20\t0\t0\t1\t\t1\t").unwrap()];
        let rader = rader(&paner, |_| Some((Gruppe::Spør, Merke::Sett)), None, true);
        assert_eq!(rader[1], Rad::Kort(&paner[0], Merke::Sett));
    }
}
