//! Rekkefølgen i lista. Måten til nerdr har de aktive først og de ledige
//! under. Lista kan også grupperes etter mappe, eller stå etter når
//! Claude-ene sist gjorde noe.

use std::collections::HashSet;

use nerdr_kjerne::merke::Merke;
use nerdr_kjerne::paner::Pane;


#[derive(Debug, PartialEq)]
pub enum Rad<'a> {
    Tom,
    Overskrift(Overskrift),
    Pane(&'a Pane, Merke),
    /// Et prosjekt brettet sammen til én rad.
    Brettet(String, Vec<(&'a Pane, Merke)>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Overskrift {
    Del(Del),
    /// Stien til repoet, eller mappa når den ikke er i et repo.
    Mappe(String),
    /// Over lista sortert etter aktivitet. Den har bare streken, så kortene
    /// starter på samme rad som i de andre sorteringene.
    Alle,
}

impl Overskrift {
    pub fn tittel(&self) -> &str {
        match self {
            Overskrift::Del(d) => tittel(*d),
            Overskrift::Mappe(sti) => navn(sti),
            Overskrift::Alle => "",
        }
    }
}

/// Bare navnet, fordi stien til repoet er en annen enn den et worktree av det
/// ligger i, og da ville overskriften vært feil for Claude-ene der.
pub fn navn(sti: &str) -> &str {
    sti.rsplit('/').next().unwrap_or(sti)
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

/// En gruppe for hver mappe, og panene i hver i den rekkefølgen de ble laget,
/// så ingen flytter seg når tilstanden endres. Et prosjekt i `brettet` er én
/// rad.
pub fn etter_mappe<'a>(
    paner: &'a [Pane],
    brettet: &HashSet<String>,
    mappe: impl Fn(&Pane) -> Option<(Merke, String)>,
) -> Vec<Rad<'a>> {
    let mut plassert: Vec<(&Pane, Merke, String)> = paner.iter().filter_map(|p| mappe(p).map(|(m, navn)| (p, m, navn))).collect();
    plassert.sort_by_key(|(p, _, _)| nummer(p));
    let mut mapper: Vec<(String, Vec<(&Pane, Merke)>)> = Vec::new();
    for (pane, merke, navn) in plassert {
        match mapper.iter_mut().find(|(n, _)| *n == navn) {
            Some((_, paner)) => paner.push((pane, merke)),
            None => mapper.push((navn, vec![(pane, merke)])),
        }
    }
    let mut rader = Vec::new();
    for (i, (navn, paner)) in mapper.into_iter().enumerate() {
        if i > 0 {
            rader.push(Rad::Tom);
        }
        if brettet.contains(&navn) {
            rader.push(Rad::Brettet(navn, paner));
        } else {
            rader.push(Rad::Overskrift(Overskrift::Mappe(navn)));
            rader.extend(paner.into_iter().map(|(p, m)| Rad::Pane(p, m)));
        }
    }
    rader
}

/// tmux teller opp pane-id-en for hver nye pane, så den sier hvilken som ble
/// laget først.
fn nummer(pane: &Pane) -> u64 {
    pane.id.trim_start_matches('%').parse().unwrap_or(u64::MAX)
}

/// Én gruppe, med den som sist endret tilstand, øverst. En som ikke har
/// meldt noe ennå, har nettopp startet.
pub fn etter_aktivitet<'a>(paner: &'a [Pane], plass: impl Fn(&Pane) -> Option<Plass>) -> Vec<Rad<'a>> {
    let mut plassert: Vec<(&Pane, Plass)> = paner.iter().filter_map(|p| Some((p, plass(p)?))).collect();
    plassert.sort_by_key(|(_, pl)| std::cmp::Reverse(pl.siden.unwrap_or(u64::MAX)));
    med_overskrifter(plassert.into_iter().map(|(p, pl)| (p, pl.merke, Some(Overskrift::Alle))))
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
        Pane::fra_linje(&format!("$0\ts\t@0\t0\t{id}\tt\t80\t20\t80\t20\t0\t0\t1\t\t1\t\t\t\t\t")).unwrap()
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
    fn en_overskrift_for_hver_mappe() {
        let paner = [pane("%1"), pane("%2"), pane("%3")];
        let rader = etter_mappe(&paner, &HashSet::new(), |p| match p.id.as_str() {
            "%2" => Some((Merke::Spør, "dotfiles".into())),
            _ => Some((Merke::Sett, "Nerdr".into())),
        });
        assert_eq!(
            rader,
            [
                Rad::Overskrift(Overskrift::Mappe("Nerdr".into())),
                Rad::Pane(&paner[0], Merke::Sett),
                Rad::Pane(&paner[2], Merke::Sett),
                Rad::Tom,
                Rad::Overskrift(Overskrift::Mappe("dotfiles".into())),
                Rad::Pane(&paner[1], Merke::Spør),
            ]
        );
    }

    #[test]
    fn prosjektene_og_panene_i_den_rekkefølgen_de_ble_laget() {
        let paner = [pane("%12"), pane("%3"), pane("%7")];
        let rader = etter_mappe(&paner, &HashSet::new(), |p| match p.id.as_str() {
            "%7" => Some((Merke::Spør, "nerdr".into())),
            _ => Some((Merke::Sett, "awl".into())),
        });
        assert_eq!(rekkefølge(&rader), ["%3", "%12", "%7"]);
    }

    #[test]
    fn et_brettet_prosjekt_er_én_rad() {
        let paner = [pane("%1"), pane("%2"), pane("%3")];
        let brettet = HashSet::from(["awl".to_string()]);
        let rader = etter_mappe(&paner, &brettet, |p| match p.id.as_str() {
            "%2" => Some((Merke::Spør, "nerdr".into())),
            "%1" => Some((Merke::Jobber, "awl".into())),
            _ => Some((Merke::Sett, "awl".into())),
        });
        assert_eq!(
            rader,
            [
                Rad::Brettet("awl".into(), vec![(&paner[0], Merke::Jobber), (&paner[2], Merke::Sett)]),
                Rad::Tom,
                Rad::Overskrift(Overskrift::Mappe("nerdr".into())),
                Rad::Pane(&paner[1], Merke::Spør),
            ]
        );
    }

    #[test]
    fn sist_aktiv_først_under_én_strek() {
        let paner = [pane("%1"), pane("%2"), pane("%3")];
        let rader = etter_aktivitet(&paner, |p| match p.id.as_str() {
            "%1" => plass(Merke::Spør, 10),
            "%2" => plass(Merke::Sett, 30),
            _ => Some(Plass::fra(Merke::Starter, None)),
        });
        assert_eq!(rekkefølge(&rader), ["%3", "%2", "%1"]);
        assert_eq!(rader[0], Rad::Overskrift(Overskrift::Alle));
        assert!(!rader[1..].iter().any(|r| matches!(r, Rad::Overskrift(_) | Rad::Tom)));
    }
}
