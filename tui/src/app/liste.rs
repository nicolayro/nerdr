//! Rekkefølgen i lista, og hva som hører til hvert kort.

use super::App;
use nerdr_kjerne::merke::Merke;
use nerdr_kjerne::paner::Pane;
use crate::{Fokus, grupper, tekst};
use crate::agent::Agent;
use crate::grupper::{Del, Plassering, Rad};
use crate::utseende::Orden;

impl App {
    /// Prosjektet til kortet som er markert, når lista er ordnet etter mappe.
    pub(crate) fn valgt_prosjekt(&self) -> Option<String> {
        let id = self.markert().filter(|_| self.utseende.orden == Orden::Mappe)?;
        self.agent_paner.iter().find(|p| p.id == id).map(|p| self.prosjekt(p))
    }

    pub(crate) fn brett(&mut self, sammen: bool) -> bool {
        let Some(prosjekt) = self.valgt_prosjekt() else { return false };
        if sammen { self.brettet.insert(prosjekt) } else { self.brettet.remove(&prosjekt) }
    }

    /// Flytter pekeren, ikke det som vises.
    pub(crate) fn flytt(&mut self, steg: isize) {
        let fra = self.peker.clone();
        if let Some(neste) = self.nabo(fra.as_deref(), steg) {
            self.peker = Some(neste.id);
            self.liste_følger = true;
        }
    }

    /// Claude-en `steg` kort fra `fra` i lista, stoppet ved endene.
    pub(crate) fn nabo(&self, fra: Option<&str>, steg: isize) -> Option<Pane> {
        let paner = self.i_rekkefølge();
        if paner.is_empty() {
            return None;
        }
        let nå = fra.and_then(|id| paner.iter().position(|p| p.id == id)).unwrap_or(0);
        let ny = (nå as isize + steg).clamp(0, paner.len() as isize - 1) as usize;
        (ny != nå).then(|| paner[ny].clone())
    }

    /// Kortet som er markert i lista: pekeren når tastene går dit, ellers den
    /// valgte.
    pub(crate) fn markert(&self) -> Option<&str> {
        match self.fokus {
            Fokus::Liste => self.peker.as_deref(),
            Fokus::Pane => self.valgt.as_ref().map(|p| p.id.as_str()),
        }
    }

    /// Velger kortet pekeren står på, og går inn i det, også når et annet
    /// vindu i sesjonen vises, som `choose-tree` i tmux.
    pub(crate) fn gå_inn(&mut self) {
        let pekt = self.peker.as_ref().and_then(|id| self.agent_paner.iter().find(|p| &p.id == id)).cloned();
        if let Some(pane) = pekt {
            let vises = self.valgt.as_ref().is_some_and(|v| v.id == pane.id) && Some(&pane.vindu_id) == self.vist.as_ref();
            if !vises {
                self.velg(pane);
            }
        }
        self.bytt_fokus(Fokus::Pane);
    }

    pub(crate) fn merke(&self, pane: &Pane) -> Option<Merke> {
        let agent = self.agenter.get(&pane.id)?;
        Some(agent.merke(self.sett.get(&pane.id).copied().unwrap_or(0)))
    }

    pub(crate) fn plass(&self, pane: &Pane) -> Option<Plassering> {
        let merke = self.merke(pane)?;
        let del = Del::fra(merke);
        // Som tiden på kortet, så de ledige ikke flytter seg når du har vært inne i en.
        let siden = self.agenter.get(&pane.id).and_then(Agent::endret);
        Some(match self.låst.iter().chain(&self.forlatt).find(|(id, _)| *id == pane.id) {
            // Den du er inne i, blir ikke ledig før du er ute og registeret
            // er lest. En ny Claude regnes som sett fra den dukker opp i
            // registeret. Settes en ledig i gang, havner den øverst blant de
            // aktive.
            Some((_, låst)) => {
                let del = del.min(låst.del);
                Plassering { del, merke, siden: if del == låst.del { låst.siden } else { None } }
            }
            _ => Plassering { del, merke, siden },
        })
    }

    pub(crate) fn rader(&self) -> Vec<Rad<'_>> {
        let paner = &self.agent_paner;
        match self.utseende.orden {
            Orden::Nerdr => grupper::rader(paner, |p| self.plass(p)),
            Orden::Mappe => {
                grupper::etter_mappe(paner, &self.brettet, |p| Some((self.plass(p)?.merke, self.prosjekt(p))))
            }
            Orden::Aktivitet => grupper::etter_aktivitet(paner, |p| self.plass(p)),
        }
    }

    /// Repoet panen jobber i, eller mappa når den ikke er i et repo.
    pub(crate) fn sted(&self, pane: &Pane) -> String {
        tekst::sted(pane, self.mappe(pane), self.repoer.get(&pane.id))
    }

    pub(crate) fn mappe<'a>(&'a self, pane: &Pane) -> Option<&'a std::path::Path> {
        self.agenter.get(&pane.id)?.mappe()
    }

    /// Stien til repoet panen jobber i, så et worktree hører til repoet det
    /// er laget fra, og to repoer med samme navn er to prosjekter.
    pub(crate) fn prosjekt(&self, pane: &Pane) -> String {
        match (self.repoer.get(&pane.id), self.mappe(pane)) {
            (Some(repo), _) => tekst::med_tilde(&repo.rot),
            (None, Some(mappe)) => tekst::med_tilde(mappe),
            (None, None) => self.sted(pane),
        }
    }

    /// Hvor panen jobber, når det ikke er i mappa til prosjektet: stien inne
    /// i repoet, eller hele stien til et worktree.
    pub(crate) fn undermappe(&self, pane: &Pane) -> Option<String> {
        let (repo, mappe) = (self.repoer.get(&pane.id)?, self.mappe(pane)?);
        if mappe == repo.rot {
            return None;
        }
        Some(mappe.strip_prefix(&repo.rot).map_or_else(|_| tekst::med_tilde(mappe), |inne| inne.display().to_string()))
    }

    pub(crate) fn i_rekkefølge(&self) -> Vec<Pane> {
        let rader = self.rader();
        rader
            .into_iter()
            .filter_map(|r| match r {
                Rad::Pane(p, _) => Some(p.clone()),
                // Pekeren står for prosjektet, så du kan flytte deg forbi det.
                Rad::Brettet(_, paner) => {
                    let valgt = paner.iter().find(|(p, _)| self.peker.as_ref() == Some(&p.id));
                    valgt.or(paner.first()).map(|(p, _)| (*p).clone())
                }
                _ => None,
            })
            .collect()
    }
}
