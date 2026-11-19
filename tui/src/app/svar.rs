//! Svarene fra tmux og hva de gjør med appen.

use super::App;
use nerdr_kjerne::{git, start, tmux};
use nerdr_kjerne::kontroll::{sitat, synkronisert_etter};
use nerdr_kjerne::paner::{self, Pane};
use nerdr_kjerne::sgr;
use std::time::{Duration, Instant};
use crate::{Fokus, Svar, UTEN_CLAUDE_MAKS, nå_ms, taster, tegn, tmuxtaster};
use crate::agent::Agent;

impl App {
    pub(crate) fn output(&mut self, pane: String, data: &str) {
        self.utdatert = true;
        let søk = self.output_hale.remove(&pane).unwrap_or_default() + data;
        match synkronisert_etter(&søk) {
            Some(true) => {
                self.synk.entry(pane.clone()).or_insert_with(Instant::now);
            }
            Some(false) => {
                self.synk.remove(&pane);
            }
            None => {}
        }
        // Markøren er 11 tegn, så de siste 10 holder for å fange en som er
        // delt mellom to biter.
        let fra = søk.char_indices().rev().nth(9).map_or(0, |(i, _)| i);
        self.output_hale.insert(pane, søk[fra..].to_string());
    }

    pub(crate) fn håndter_svar(&mut self, svar: Svar, ok: bool, linjer: Vec<String>) -> bool {
        match svar {
            Svar::Paner => {
                self.lister = false;
                let aktivert = self.aktivert.take();
                self.sist_listet = Some(Instant::now());
                self.forlatt = None;
                let alle: Vec<Pane> = linjer.iter().filter_map(|l| Pane::fra_linje(l)).collect();
                self.les_agenter(&alle, ok);
                let tom = !alle.iter().any(|p| self.agenter.get(&p.id).and_then(Agent::claude).is_some());
                self.utenfor_tmux = if tom { self.ut.utenfor_tmux() } else { 0 };
                self.husk_sett(&alle);
                if ok {
                    self.fargelagt.retain(|id| alle.iter().any(|p| &p.id == id));
                    for pane in &alle {
                        if !self.fargelagt.contains(&pane.id) {
                            self.gi_farger(&pane.id);
                        }
                    }
                    self.rydd(&alle);
                }
                self.repoer = self
                    .agenter
                    .iter()
                    .filter_map(|(id, a)| Some((id.clone(), git::les(a.mappe()?)?)))
                    .collect();
                self.agent_paner = alle.iter().filter(|p| ok && self.agenter.contains_key(&p.id)).cloned().collect();
                self.størrelser.retain(|vindu, _| alle.iter().any(|p| &p.vindu_id == vindu));
                self.alle = alle;
                let valgt = self.valgt.as_ref().and_then(|v| self.agent_paner.iter().find(|p| p.id == v.id));
                match valgt.cloned() {
                    // Oppdatert, så størrelsen regnes fra det tmux sier nå.
                    Some(pane) => {
                        self.valgt = Some(pane);
                        self.oppdater_vindu(aktivert.as_deref());
                    }
                    None => {
                        // Tastene du er midt i, skal ikke gå til en annen pane.
                        self.bytt_fokus(Fokus::Liste);
                        if let Some(pane) = self.i_rekkefølge().first().cloned() {
                            self.velg(pane);
                        } else {
                            self.valgt = None;
                            self.vindu.clear();
                        }
                    }
                }
                if !self.peker.as_ref().is_some_and(|id| self.agent_paner.iter().any(|p| &p.id == id)) {
                    self.peker = self.valgt.as_ref().map(|p| p.id.clone());
                }
                let ny = self.gå_inn_i.as_ref().and_then(|id| self.agent_paner.iter().find(|p| &p.id == id));
                if let Some(pane) = ny.cloned() {
                    self.gå_inn_i = None;
                    // Er du inne i en annen Claude, skal ikke tastene dine gå et
                    // annet sted midt i en setning.
                    if self.fokus == Fokus::Liste {
                        self.velg(pane);
                        self.bytt_fokus(Fokus::Pane);
                    }
                }
                if std::mem::take(&mut self.list_igjen) {
                    self.list_paner();
                }
                true
            }
            Svar::Bilde(id) => {
                if ok && self.vindu.iter().any(|p| p.id == id) {
                    self.bilder.insert(id, sgr::tolk(&linjer));
                }
                false
            }
            Svar::Kopi { markering, fra, bredde } => {
                let tekst = markering.tekst(&sgr::tolk(&linjer), fra, bredde);
                if ok && !tekst.is_empty() && self.ut.kopier(&tekst) {
                    self.kopiert = Some(Instant::now());
                }
                true
            }
            Svar::Markør => {
                self.henter = false;
                let felt: Vec<&str> = linjer.first().map_or("", |l| l.as_str()).split('\t').collect();
                self.aktiv_pane.markør = match felt.as_slice() {
                    [x, y, "1", ..] => x.parse().ok().zip(y.parse().ok()),
                    _ => None,
                };
                self.aktiv_pane.mus = felt.get(3) == Some(&"1");
                self.aktiv_pane.alternativ = felt.get(4) == Some(&"1");
                self.aktiv_pane.bevegelse = felt.get(6) == Some(&"1");
                let historie: u32 = felt.get(5).and_then(|h| h.parse().ok()).unwrap_or(0);
                // Det du leser, blir stående når ny tekst skyver historikken opp,
                // og i kopimodus også nederst.
                let rull = match self.aktiv_pane.rull {
                    _ if self.aktiv_pane.alternativ => 0,
                    0 if self.aktiv_pane.kopimodus.is_none() => 0,
                    rull => rull.saturating_add(historie.saturating_sub(self.aktiv_pane.historie)).min(historie),
                };
                self.aktiv_pane.historie = historie;
                if rull != self.aktiv_pane.rull {
                    self.aktiv_pane.rull = rull;
                    self.utdatert = true;
                }
                true
            }
            Svar::NySesjon { navn, forsøk, mappe, claude_kommando } if !ok && forsøk < start::MAKS_FORSØK => {
                let forsøk = forsøk + 1;
                let linje = tmux::linje(&start::ny_sesjon(&start::nummerert(&navn, forsøk), &mappe));
                self.send(&linje, Svar::NySesjon { navn, forsøk, mappe, claude_kommando });
                false
            }
            Svar::NySesjon { mappe, claude_kommando, .. } => {
                let Some(id) = linjer.into_iter().next().filter(|_| ok) else { return false };
                // Før `claude` skrives inn, så den finner temaet når den starter.
                self.gi_farger(&id);
                self.send(&tmux::linje(&start::merk(&id)), Svar::Ingenting);
                for kommando in start::skriv_inn(&id, &claude_kommando) {
                    self.send(&tmux::linje(&kommando), Svar::Ingenting);
                }
                self.agenter.insert(id.clone(), Agent::Starter { mappe, startet: Instant::now() });
                self.gå_inn_i = Some(id);
                self.list_paner();
                false
            }
            Svar::Liste => {
                let verdi = linjer.first().filter(|_| ok).map_or("", |l| l.as_str());
                let mut ord = verdi.split_whitespace();
                if let Some(bredde) = ord.next().and_then(|b| b.parse::<u16>().ok()) {
                    self.liste_bredde = bredde.max(tegn::LISTE_MIN);
                    self.liste_stripe = ord.next() == Some("stripe");
                }
                true
            }
            Svar::Utseende => {
                if let Some(tekst) = linjer.first().filter(|_| ok) {
                    self.utseende.les(tekst);
                    self.utdatert = true;
                }
                true
            }
            Svar::Prefiks => {
                if let Some(prefiks) = linjer.first().filter(|_| ok).and_then(|l| taster::Prefiks::fra_tmux(l)) {
                    self.prefiks = prefiks;
                }
                false
            }
            Svar::Bindinger => {
                if ok {
                    let gjentatid = self.bindinger.gjentatid;
                    self.bindinger = tmuxtaster::Bindinger::ny(&nerdr_kjerne::bindinger::les(&linjer));
                    self.bindinger.gjentatid = gjentatid;
                }
                false
            }
            Svar::Root => {
                if ok {
                    self.root = tmuxtaster::Bindinger::root(&nerdr_kjerne::bindinger::les(&linjer));
                }
                false
            }
            Svar::Gjentatid => {
                if let Some(ms) = linjer.first().filter(|_| ok).and_then(|l| l.parse().ok()) {
                    self.bindinger.gjentatid = Duration::from_millis(ms);
                }
                false
            }
            Svar::NyttVindu => {
                let Some(id) = linjer.into_iter().next().filter(|_| ok) else { return false };
                self.vis_vindu(id);
                self.bytt_fokus(Fokus::Pane);
                self.list_paner();
                true
            }
            Svar::Ingenting => false,
        }
    }

    pub(crate) fn hent(&mut self) {
        let Some(aktiv) = self.aktiv().map(|p| p.id.clone()) else {
            // Ellers ber `tid_til_henting` om en ny henting med en gang, i hver runde.
            self.utdatert = false;
            return;
        };
        let mut ider: Vec<String> = self.bokser().into_iter().map(|b| b.pane).collect();
        if ider.is_empty() {
            ider.push(aktiv.clone());
        }
        for id in ider {
            // Fra -S og ned til bunnen er flere linjer enn panen, og tegningen
            // tar de øverste.
            let fra = if id == aktiv && self.aktiv_pane.rull > 0 { format!(" -S -{}", self.aktiv_pane.rull) } else { String::new() };
            self.send(&format!("capture-pane -p -e -N -t {}{fra}", sitat(&id)), Svar::Bilde(id));
        }
        let id = sitat(&aktiv);
        let felt = sitat(
            "#{cursor_x}\t#{cursor_y}\t#{cursor_flag}\t#{&&:#{mouse_any_flag},#{mouse_sgr_flag}}\t\
             #{alternate_on}\t#{history_size}\t#{mouse_all_flag}",
        );
        self.send(&format!("display-message -p -t {id} {felt}"), Svar::Markør);
        self.utdatert = false;
        self.henter = true;
        self.sist_hentet = Instant::now();
    }

    /// Registeret vinner, så en Claude som er kommet dit, ikke lenger starter.
    /// Uten en liste vet vi ikke om panene er borte, så de som starter, og
    /// restene blir stående.
    pub(crate) fn les_agenter(&mut self, alle: &[Pane], ok: bool) {
        let forrige = std::mem::take(&mut self.agenter);
        self.agenter = self.ut.register().into_iter().map(|(id, c)| (id, Agent::Claude(c))).collect();
        let inne = |id: &str| self.fokus == Fokus::Pane && self.valgt.as_ref().is_some_and(|v| v.id == id);
        for (id, agent) in forrige {
            let behold = match &agent {
                Agent::Starter { startet, .. } => {
                    !ok || (alle.iter().any(|p| p.id == id) && (startet.elapsed() < self.starter_maks || inne(&id)))
                }
                Agent::Rest { .. } => !ok,
                Agent::Claude(_) => false,
            };
            if behold && !self.agenter.contains_key(&id) {
                self.agenter.insert(id, agent);
            }
        }
    }

    pub(crate) fn er_claude(&self, id: &str) -> bool {
        self.agenter.get(id).is_some_and(Agent::er_claude)
    }

    /// Sesjonene nerdr har laget, der Claude-en har vært borte en stund. Står
    /// det bare skall igjen, lukkes sesjonen. Ellers blir et av programmene
    /// et kort, så det du har åpnet, ikke blir borte for deg.
    pub(crate) fn rydd(&mut self, alle: &[Pane]) {
        let mut uten: Vec<String> = alle.iter().filter(|p| p.fra_nerdr).map(|p| p.sesjon_id.clone()).collect();
        uten.sort();
        uten.dedup();
        uten.retain(|s| !alle.iter().any(|p| &p.sesjon_id == s && self.er_claude(&p.id)));
        self.uten_claude.retain(|s, _| uten.contains(s));
        for sesjon in uten {
            let siden = *self.uten_claude.entry(sesjon.clone()).or_insert_with(Instant::now);
            if siden.elapsed() < UTEN_CLAUDE_MAKS {
                continue;
            }
            let programmer: Vec<&Pane> = alle.iter().filter(|p| p.sesjon_id == sesjon && !p.bare_skall()).collect();
            let Some(første) = programmer.first() else {
                self.send(&format!("kill-session -t {}", sitat(&sesjon)), Svar::Ingenting);
                continue;
            };
            let mut navn: Vec<&str> = programmer.iter().map(|p| p.kommando.as_str()).collect();
            navn.dedup();
            let rest = Agent::Rest { mappe: første.mappe.clone(), programmer: navn.join(", ") };
            self.agenter.insert(første.id.clone(), rest);
        }
    }

    /// Tar inn det tmux har for panene der en Claude kjører. En pane som
    /// ingen nerdr har sett før, får tiden nå, også i tmux, så det den blir
    /// ferdig med mens nerdr er lukket, venter på deg neste gang.
    pub(crate) fn husk_sett(&mut self, alle: &[Pane]) {
        self.sett.retain(|id, _| self.agenter.get(id).and_then(Agent::claude).is_some());
        let nå = nå_ms();
        for pane in alle {
            if self.agenter.get(&pane.id).and_then(Agent::claude).is_none() {
                continue;
            }
            match (self.sett.get(&pane.id).copied(), pane.sett) {
                (None, None) => self.merk_sett(&pane.id, nå),
                (her, i_tmux) => {
                    self.sett.insert(pane.id.clone(), her.max(i_tmux).unwrap_or(nå));
                }
            }
        }
    }

    pub(crate) fn merk_sett(&mut self, id: &str, tid: u64) {
        self.sett.insert(id.to_string(), tid);
        self.send(&format!("set-option -p -t {} {} {tid}", sitat(id), paner::SETT), Svar::Ingenting);
    }
}
