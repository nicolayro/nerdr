//! Rekkefølgen i lista: de som venter på deg først, så de som jobber, så de
//! som er ferdige og sett.

use nerdr_kjerne::merke::{Gruppe, Merke};
use nerdr_kjerne::paner::Pane;

#[derive(Debug, PartialEq)]
pub enum Rad<'a> {
    Tom,
    Strek,
    Overskrift(Gruppe),
    Pane(&'a Pane, Merke),
}

pub fn glyf(merke: Merke) -> &'static str {
    match merke {
        Merke::Spør => "●",
        Merke::Ulest => "◆",
        Merke::Jobber => "◐",
        Merke::Sett => "○",
        Merke::Starter => "◌",
    }
}

pub fn tittel(gruppe: Gruppe) -> &'static str {
    match gruppe {
        Gruppe::Venter => "VENTER",
        Gruppe::Jobber => "JOBBER",
        Gruppe::Ferdig => "FERDIG",
    }
}

/// Hvor en pane står i lista.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plass {
    pub gruppe: Gruppe,
    pub merke: Merke,
    /// Da panen kom inn i gruppen, i millisekunder siden 1970.
    pub siden: Option<u64>,
}

impl Plass {
    pub fn fra(merke: Merke, siden: Option<u64>) -> Plass {
        Plass { gruppe: merke.gruppe(), merke, siden }
    }
}

/// Panene gruppert, med en overskrift for hver gruppe som ikke er tom. I
/// VENTER og JOBBER står den som kom inn først, øverst, så en ny legges til
/// nederst og ikke skyver ned dem du er i gang med. I FERDIG står den nyeste
/// øverst, så det er de eldste som ikke får plass. Plassen gis for seg, fordi
/// panen du er inne i, blir stående der den var selv om merket endres.
pub fn rader<'a>(paner: &'a [Pane], plass: impl Fn(&Pane) -> Option<Plass>) -> Vec<Rad<'a>> {
    let mut plassert: Vec<(&Pane, Plass)> = paner.iter().filter_map(|p| Some((p, plass(p)?))).collect();
    // Stabil, så like tider beholder rekkefølgen fra tmux.
    plassert.sort_by(|(_, a), (_, b)| {
        a.gruppe.cmp(&b.gruppe).then_with(|| match a.gruppe {
            Gruppe::Ferdig => b.siden.cmp(&a.siden),
            _ => a.siden.unwrap_or(u64::MAX).cmp(&b.siden.unwrap_or(u64::MAX)),
        })
    });

    let mut rader = Vec::new();
    let mut forrige = None;
    for (pane, Plass { gruppe: g, merke: m, .. }) in plassert {
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

    fn pane(id: &str) -> Pane {
        Pane::fra_linje(&format!("$0\ts\t@0\t0\t{id}\tt\t80\t20\t80\t20\t0\t0\t1\t\t1\t\t")).unwrap()
    }

    fn plass(merke: Merke, siden: u64) -> Option<Plass> {
        Some(Plass { gruppe: merke.gruppe(), merke, siden: Some(siden) })
    }

    #[test]
    fn grupperer_med_overskrifter_og_strek_mellom_panene() {
        let paner = [pane("%1"), pane("%2"), pane("%3")];
        let rader = rader(&paner, |p| match p.id.as_str() {
            "%1" => plass(Merke::Sett, 0),
            "%2" => plass(Merke::Ulest, 0),
            _ => plass(Merke::Spør, 0),
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
    fn eldste_først_i_venter_og_nyeste_først_i_ferdig() {
        let paner = [pane("%1"), pane("%2"), pane("%3"), pane("%4")];
        let rader = rader(&paner, |p| match p.id.as_str() {
            "%1" => plass(Merke::Spør, 20),
            "%2" => plass(Merke::Ulest, 10),
            "%3" => plass(Merke::Sett, 10),
            _ => plass(Merke::Sett, 20),
        });
        let rekkefølge: Vec<&str> = rader
            .iter()
            .filter_map(|r| match r {
                Rad::Pane(p, _) => Some(p.id.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(rekkefølge, ["%2", "%1", "%4", "%3"]);
    }

    #[test]
    fn gruppen_kan_avvike_fra_merket() {
        let paner = [pane("%1")];
        let rader = rader(&paner, |_| Some(Plass { gruppe: Gruppe::Jobber, merke: Merke::Sett, siden: None }));
        assert_eq!(rader, [Rad::Overskrift(Gruppe::Jobber), Rad::Pane(&paner[0], Merke::Sett)]);
    }
}
