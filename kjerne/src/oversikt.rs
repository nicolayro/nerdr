//! Sesjonsoversikten: én rad per Claude, pluss én per sesjon uten noen,
//! gruppert på repo.

use std::collections::HashMap;

use crate::git::Git;
use crate::tilstand::{self, Tilstand, TITTEL_GLYF};

/// Formatet til `list-panes -a -F`. Rekkefølgen må stemme med `Pane::fra_linje`.
pub const FELT: &str = "#{session_id}\t#{session_name}\t#{window_id}\t#{window_index}\t\
#{pane_id}\t#{pane_current_path}\t#{pane_title}\t#{@claude_state}\t#{@claude_sidebar}\t\
#{window_width}\t#{window_height}\t#{pane_width}\t#{pane_height}";

#[derive(Clone, Debug, PartialEq)]
pub struct Pane {
    pub sesjon_id: String,
    pub sesjon: String,
    pub vindu_id: String,
    pub vindu: String,
    pub id: String,
    pub sti: String,
    pub tittel: String,
    /// `@claude_state`, satt av Notification-hooken.
    pub hook_state: String,
    /// Sidebar-kolonnene og plassholderne fra tmux-oppsettet.
    pub er_sidebar: bool,
    pub vindu_bredde: u16,
    pub vindu_høyde: u16,
    pub bredde: u16,
    pub høyde: u16,
}

impl Pane {
    pub fn fra_linje(linje: &str) -> Option<Pane> {
        let f: Vec<&str> = linje.split('\t').collect();
        let [sesjon_id, sesjon, vindu_id, vindu, id, sti, tittel, state, sidebar, vb, vh, b, h] =
            f.as_slice()
        else {
            return None;
        };
        Some(Pane {
            sesjon_id: sesjon_id.to_string(),
            sesjon: sesjon.to_string(),
            vindu_id: vindu_id.to_string(),
            vindu: vindu.to_string(),
            id: id.to_string(),
            sti: sti.to_string(),
            tittel: tittel.to_string(),
            hook_state: state.to_string(),
            er_sidebar: !sidebar.is_empty(),
            vindu_bredde: vb.parse().ok()?,
            vindu_høyde: vh.parse().ok()?,
            bredde: b.parse().ok()?,
            høyde: h.parse().ok()?,
        })
    }

    pub fn er_claude(&self) -> bool {
        self.tittel.starts_with(TITTEL_GLYF)
    }
}

#[derive(Clone, Debug)]
pub struct Rad {
    pub pane: Pane,
    pub worktree: String,
    pub branch: String,
    pub tilstand: Tilstand,
    pub status: String,
    pub oppgave: String,
    pub spørsmål: String,
    pub kontekst: String,
    /// Sesjonsnavnet, med vindu bak når sesjonen har flere Clauder.
    pub etikett: String,
}

impl Rad {
    /// Det raden viser som hovedtekst: det som blokkerer, ellers oppgaven.
    pub fn hovedtekst(&self) -> &str {
        [&self.spørsmål, &self.oppgave, &self.etikett]
            .into_iter()
            .find(|t| !t.is_empty())
            .map_or("", |t| t.as_str())
    }
}

#[derive(Clone, Debug)]
pub struct Gruppe {
    pub repo: String,
    pub rader: Vec<Rad>,
}

/// `skjermer` er pane-id → de siste linjene av skjermbildet, for Claude-panene.
pub fn bygg(paner: &[Pane], skjermer: &HashMap<String, String>, git: &mut Git) -> Vec<Gruppe> {
    let mut per_sesjon: Vec<(&str, Vec<&Pane>)> = Vec::new();
    for pane in paner.iter().filter(|p| !p.er_sidebar) {
        match per_sesjon.iter_mut().find(|(s, _)| *s == pane.sesjon_id) {
            Some((_, alle)) => alle.push(pane),
            None => per_sesjon.push((&pane.sesjon_id, vec![pane])),
        }
    }

    let mut grupper: Vec<Gruppe> = Vec::new();
    for (_, alle) in per_sesjon {
        let claude: Vec<&Pane> = alle.iter().copied().filter(|p| p.er_claude()).collect();
        let valgt = if claude.is_empty() { vec![alle[0]] } else { claude };
        let flere = valgt.len() > 1;
        for pane in valgt {
            let lesning = if pane.er_claude() {
                let skjerm = skjermer.get(&pane.id).map_or("", |s| s.as_str());
                tilstand::les(skjerm, pane.hook_state == "waiting")
            } else {
                tilstand::Lesning {
                    tilstand: Tilstand::Shell,
                    status: "ledig".into(),
                    kontekst: String::new(),
                    spørsmål: String::new(),
                }
            };
            let repo = git.repo(&pane.sti);
            let rad = Rad {
                worktree: git.worktree(&pane.sti, &repo),
                branch: git.branch(&pane.sti),
                tilstand: lesning.tilstand,
                status: lesning.status,
                oppgave: tilstand::oppgave(&pane.tittel, &pane.sti),
                spørsmål: lesning.spørsmål,
                kontekst: lesning.kontekst,
                // Vindusnummeret er bare støy når sesjonen har én Claude.
                etikett: if flere {
                    format!("{}:{}", pane.sesjon, pane.vindu)
                } else {
                    pane.sesjon.clone()
                },
                pane: pane.clone(),
            };
            match grupper.iter_mut().find(|g| g.repo == repo) {
                Some(g) => g.rader.push(rad),
                None => grupper.push(Gruppe { repo, rader: vec![rad] }),
            }
        }
    }

    grupper.sort_by(|a, b| a.repo.cmp(&b.repo));
    for gruppe in &mut grupper {
        // Hovedutsjekken først, resten alfabetisk. Den kjennes på at den ikke
        // har noen worktree-merkelapp, så regelen gjelder alle repoer uten å
        // anta at utsjekken heter «main».
        gruppe
            .rader
            .sort_by(|a, b| (!a.worktree.is_empty(), &a.etikett).cmp(&(!b.worktree.is_empty(), &b.etikett)));
    }
    grupper
}

pub fn telling(grupper: &[Gruppe], tilstand: Tilstand) -> usize {
    grupper.iter().flat_map(|g| &g.rader).filter(|r| r.tilstand == tilstand).count()
}

#[cfg(test)]
mod tester {
    use super::*;

    fn pane(sesjon: &str, vindu: &str, id: &str, tittel: &str, sidebar: &str) -> String {
        format!("${sesjon}\t{sesjon}\t@{vindu}\t{vindu}\t{id}\t/finnes/ikke/{sesjon}\t{tittel}\t\t{sidebar}\t176\t38\t140\t38")
    }

    #[test]
    fn leser_list_panes_linje() {
        let p = Pane::fra_linje(&pane("main", "0", "%0", "✳ Fiks", "")).unwrap();
        assert_eq!((p.id.as_str(), p.bredde, p.vindu_bredde), ("%0", 140, 176));
        assert!(p.er_claude() && !p.er_sidebar);
        assert_eq!(Pane::fra_linje("for\tfå"), None);
    }

    #[test]
    fn sidebar_skjules_og_skall_får_én_rad() {
        let paner: Vec<Pane> = [
            pane("a", "0", "%1", "✳ En", ""),
            pane("a", "0", "%2", "sesjoner", "kolonne"),
            pane("a", "1", "%3", "✳ To", ""),
            pane("b", "0", "%4", "zsh", ""),
            pane("b", "1", "%5", "zsh", ""),
        ]
        .iter()
        .filter_map(|l| Pane::fra_linje(l))
        .collect();
        let skjermer = HashMap::from([("%1".to_string(), "Esc to cancel\n".to_string())]);
        let grupper = bygg(&paner, &skjermer, &mut Git::default());

        let rader: Vec<(&str, &str, Tilstand)> = grupper
            .iter()
            .flat_map(|g| &g.rader)
            .map(|r| (r.pane.id.as_str(), r.etikett.as_str(), r.tilstand))
            .collect();
        assert_eq!(
            rader,
            vec![
                ("%1", "a:0", Tilstand::Venter),
                ("%3", "a:1", Tilstand::Klar),
                ("%4", "b", Tilstand::Shell),
            ]
        );
        assert_eq!(telling(&grupper, Tilstand::Venter), 1);
    }
}
