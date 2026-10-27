//! Rekkefølgen i lista. Måten til nerdr har de aktive først og de ledige
//! under. Lista kan også grupperes etter mappe, eller stå etter når
//! Claude-ene sist gjorde noe.

use nerdr_kjerne::merke::Merke;
use nerdr_kjerne::paner::Pane;

#[derive(Debug, PartialEq)]
pub enum Rad<'a> {
    Tom,
    Overskrift(Overskrift),
    Pane(&'a Pane, Merke),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Overskrift {
    Del(Del),
    /// Repoet, eller mappa når den ikke er i et repo.
    Mappe(String),
}

impl Overskrift {
    pub fn tittel(&self) -> &str {
        match self {
            Overskrift::Del(d) => tittel(*d),
            Overskrift::Mappe(navn) => navn,
        }
    }
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

/// Lista til nerdr er delt i to. En Claude blir ledig når du har gått ut av den
/// uten å sette den i gang igjen, så den du er inne i, er aldri ledig.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Del {
    Aktive,
    Ledige,
}

impl Del {
    pub fn fra(merke: Merke) -> Del {
        if merke == Merke::Sett { Del::Ledige } else { Del::Aktive }
    }
}

pub fn tittel(del: Del) -> &'static str {
    match del {
        Del::Aktive => "AKTIVE",
        Del::Ledige => "LEDIGE",
    }
}

/// Hvor en pane står i lista.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plass {
    pub del: Del,
    pub merke: Merke,
    /// Da panen kom inn i delen, i millisekunder siden 1970.
    pub siden: Option<u64>,
}

impl Plass {
    pub fn fra(merke: Merke, siden: Option<u64>) -> Plass {
        Plass { del: Del::fra(merke), merke, siden }
    }
}

/// Panene gruppert, med en overskrift for hver gruppe som ikke er tom. I hver
/// gruppe står den som kom sist inn, øverst. En som ikke har meldt noe ennå,
/// har nettopp startet. Plassen gis for seg, fordi panen du er inne i, blir
/// stående der den var selv om merket endres.
pub fn rader<'a>(paner: &'a [Pane], plass: impl Fn(&Pane) -> Option<Plass>) -> Vec<Rad<'a>> {
    let mut plassert: Vec<(&Pane, Plass)> = paner.iter().filter_map(|p| Some((p, plass(p)?))).collect();
    // Stabil, så like tider beholder rekkefølgen fra tmux.
    plassert.sort_by(|(_, a), (_, b)| {
        a.del.cmp(&b.del).then_with(|| b.siden.unwrap_or(u64::MAX).cmp(&a.siden.unwrap_or(u64::MAX)))
    });
    med_overskrifter(plassert.into_iter().map(|(p, pl)| (p, pl.merke, Some(Overskrift::Del(pl.del)))))
}

/// En gruppe for hver mappe, i alfabetisk rekkefølge, og panene i hver i
/// rekkefølgen fra tmux, så ingen flytter seg når tilstanden endres.
pub fn etter_mappe<'a>(paner: &'a [Pane], mappe: impl Fn(&Pane) -> Option<(Merke, String)>) -> Vec<Rad<'a>> {
    let mut plassert: Vec<(&Pane, Merke, String)> = paner.iter().filter_map(|p| mappe(p).map(|(m, navn)| (p, m, navn))).collect();
    plassert.sort_by_cached_key(|(_, _, navn)| navn.to_lowercase());
    med_overskrifter(plassert.into_iter().map(|(p, m, navn)| (p, m, Some(Overskrift::Mappe(navn)))))
}

/// Uten grupper, med den som sist endret tilstand, øverst. En som ikke har
/// meldt noe ennå, har nettopp startet.
pub fn etter_aktivitet<'a>(paner: &'a [Pane], plass: impl Fn(&Pane) -> Option<Plass>) -> Vec<Rad<'a>> {
    let mut plassert: Vec<(&Pane, Plass)> = paner.iter().filter_map(|p| Some((p, plass(p)?))).collect();
    plassert.sort_by_key(|(_, pl)| std::cmp::Reverse(pl.siden.unwrap_or(u64::MAX)));
    med_overskrifter(plassert.into_iter().map(|(p, pl)| (p, pl.merke, None)))
}

/// En overskrift der overskriften skifter, med en tom rad over.
fn med_overskrifter<'a>(paner: impl IntoIterator<Item = (&'a Pane, Merke, Option<Overskrift>)>) -> Vec<Rad<'a>> {
    let mut rader = Vec::new();
    let mut forrige: Option<Option<Overskrift>> = None;
    for (pane, merke, overskrift) in paner {
        if forrige.as_ref() != Some(&overskrift) {
            if forrige.is_some() {
                rader.push(Rad::Tom);
            }
            if let Some(o) = &overskrift {
                rader.push(Rad::Overskrift(o.clone()));
            }
            forrige = Some(overskrift);
        }
        rader.push(Rad::Pane(pane, merke));
    }
    rader
}

#[cfg(test)]
mod tester {
    use super::*;

    fn pane(id: &str) -> Pane {
        Pane::fra_linje(&format!("$0\ts\t@0\t0\t{id}\tt\t80\t20\t80\t20\t0\t0\t1\t\t1\t\t\t")).unwrap()
    }

    fn plass(merke: Merke, siden: u64) -> Option<Plass> {
        Some(Plass::fra(merke, Some(siden)))
    }

    fn rekkefølge<'a>(rader: &'a [Rad]) -> Vec<&'a str> {
        rader
            .iter()
            .filter_map(|r| match r {
                Rad::Pane(p, _) => Some(p.id.as_str()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn grupperer_med_overskrifter() {
        let paner = [pane("%1"), pane("%2"), pane("%3")];
        let rader = rader(&paner, |p| match p.id.as_str() {
            "%1" => plass(Merke::Sett, 0),
            "%2" => plass(Merke::Ulest, 0),
            _ => plass(Merke::Spør, 0),
        });
        assert_eq!(
            rader,
            [
                Rad::Overskrift(Overskrift::Del(Del::Aktive)),
                Rad::Pane(&paner[1], Merke::Ulest),
                Rad::Pane(&paner[2], Merke::Spør),
                Rad::Tom,
                Rad::Overskrift(Overskrift::Del(Del::Ledige)),
                Rad::Pane(&paner[0], Merke::Sett),
            ]
        );
    }

    #[test]
    fn nyeste_først_i_hver_gruppe() {
        let paner = [pane("%1"), pane("%2"), pane("%3"), pane("%4")];
        let rader = rader(&paner, |p| match p.id.as_str() {
            "%1" => plass(Merke::Spør, 20),
            "%2" => plass(Merke::Ulest, 10),
            "%3" => plass(Merke::Sett, 10),
            _ => plass(Merke::Sett, 20),
        });
        assert_eq!(rekkefølge(&rader), ["%1", "%2", "%4", "%3"]);
    }

    #[test]
    fn delen_kan_avvike_fra_merket() {
        let paner = [pane("%1")];
        let rader = rader(&paner, |_| Some(Plass { del: Del::Aktive, merke: Merke::Sett, siden: None }));
        assert_eq!(rader, [Rad::Overskrift(Overskrift::Del(Del::Aktive)), Rad::Pane(&paner[0], Merke::Sett)]);
    }

    #[test]
    fn mappene_i_alfabetisk_rekkefølge_og_panene_som_i_tmux() {
        let paner = [pane("%1"), pane("%2"), pane("%3")];
        let rader = etter_mappe(&paner, |p| match p.id.as_str() {
            "%2" => Some((Merke::Spør, "awl".into())),
            _ => Some((Merke::Sett, "Nerdr".into())),
        });
        assert_eq!(
            rader,
            [
                Rad::Overskrift(Overskrift::Mappe("awl".into())),
                Rad::Pane(&paner[1], Merke::Spør),
                Rad::Tom,
                Rad::Overskrift(Overskrift::Mappe("Nerdr".into())),
                Rad::Pane(&paner[0], Merke::Sett),
                Rad::Pane(&paner[2], Merke::Sett),
            ]
        );
    }

    #[test]
    fn sist_aktiv_først_uten_overskrifter() {
        let paner = [pane("%1"), pane("%2"), pane("%3")];
        let rader = etter_aktivitet(&paner, |p| match p.id.as_str() {
            "%1" => plass(Merke::Spør, 10),
            "%2" => plass(Merke::Sett, 30),
            _ => Some(Plass::fra(Merke::Starter, None)),
        });
        assert_eq!(rekkefølge(&rader), ["%3", "%2", "%1"]);
        assert!(!rader.iter().any(|r| matches!(r, Rad::Overskrift(_) | Rad::Tom)));
    }
}
