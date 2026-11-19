//! Vinduet som vises, panene i det og størrelsen tmux får.

use super::App;
use nerdr_kjerne::oppsett;
use nerdr_kjerne::kontroll::sitat;
use nerdr_kjerne::paner::{FELT, Pane};
use ratatui::layout::Rect;
use std::collections::HashSet;
use crate::{AktivPane, Fokus, Lukke, Svar, bokser, nå_ms};
use crate::agent::Agent;
use crate::bokser::Boks;
use crate::popup::Popup;

impl App {
    /// Neste eller forrige Claude i lista, som `(` og `)` bytter sesjon i
    /// tmux. Fra panen går du inn i den.
    pub(crate) fn bla(&mut self, steg: isize) {
        let inne = self.fokus == Fokus::Pane;
        // Via lista, så panen du forlater, merkes som sett.
        self.bytt_fokus(Fokus::Liste);
        let fra = self.valgt.as_ref().map(|p| p.id.clone());
        if let Some(neste) = self.nabo(fra.as_deref(), steg) {
            self.velg(neste);
        }
        if inne {
            self.bytt_fokus(Fokus::Pane);
        }
    }

    /// Vinduene i sesjonen som vises, med én pane fra hvert, etter nummer.
    pub(crate) fn faner(&self) -> Vec<&Pane> {
        let Some(sesjon) = self.vindu.first().map(|p| &p.sesjon_id) else { return Vec::new() };
        let mut sett = HashSet::new();
        let mut faner: Vec<&Pane> =
            self.alle.iter().filter(|p| &p.sesjon_id == sesjon && sett.insert(&p.vindu_id)).collect();
        faner.sort_by_key(|p| p.vindu.parse::<u32>().unwrap_or(u32::MAX));
        faner
    }

    pub(crate) fn neste_vindu(&mut self, steg: isize) {
        let faner: Vec<String> = self.faner().iter().map(|p| p.vindu_id.clone()).collect();
        let Some(i) = faner.iter().position(|v| Some(v) == self.vist.as_ref()) else { return };
        let neste = (i as isize + steg).rem_euclid(faner.len() as isize) as usize;
        self.vis_vindu(faner[neste].clone());
    }

    /// Viser et annet vindu. Står det en Claude i det, blir den valgt i lista.
    pub(crate) fn vis_vindu(&mut self, vindu: String) {
        if self.vist.as_ref() == Some(&vindu) {
            return;
        }
        let inne = self.fokus == Fokus::Pane;
        // Via lista, så Claude-en du forlater, merkes som sett.
        self.bytt_fokus(Fokus::Liste);
        if let Some(claude) = self.claude_paner.iter().find(|p| p.vindu_id == vindu).cloned() {
            self.velg(claude);
        } else {
            let sesjon = self.alle.iter().find(|p| p.vindu_id == vindu).map(|p| p.sesjon_id.clone());
            self.synk_størrelse_for(&vindu);
            if let Some(sesjon) = sesjon.filter(|s| self.vindu.first().map(|p| &p.sesjon_id) != Some(s)) {
                self.send(&format!("switch-client -t {}", sitat(&sesjon)), Svar::Ingenting);
            }
            self.send(&format!("select-window -t {}", sitat(&vindu)), Svar::Ingenting);
            self.vis(vindu, None);
        }
        if inne {
            self.bytt_fokus(Fokus::Pane);
        }
    }

    pub(crate) fn vis(&mut self, vindu: String, aktiv: Option<&str>) {
        if self.vist.as_ref() != Some(&vindu) {
            self.forrige_vindu = self.vist.replace(vindu);
        }
        self.bilder.clear();
        self.synk.clear();
        self.output_hale.clear();
        self.oppdater_vindu(aktiv);
        self.nullstill_aktiv();
    }

    /// Er en liste alt på vei, kan den være bestilt før endringen, så det
    /// listes en gang til når den kommer.
    pub(crate) fn list_paner(&mut self) {
        if self.lister {
            self.list_igjen = true;
        } else {
            self.lister = true;
            self.send(&format!("list-panes -a -F {}", sitat(FELT)), Svar::Paner);
        }
    }

    pub(crate) fn velg(&mut self, pane: Pane) {
        self.synk_størrelse_for(&pane.vindu_id);
        if self.valgt.as_ref().map(|p| &p.sesjon_id) != Some(&pane.sesjon_id) {
            self.send(&format!("switch-client -t {}", sitat(&pane.sesjon_id)), Svar::Ingenting);
        }
        // Vinduet må være sesjonens aktive for å få størrelsen fra denne
        // klienten. Det bytter også vindu for andre klienter i samme sesjon.
        self.send(&format!("select-window -t {}", sitat(&pane.vindu_id)), Svar::Ingenting);
        self.send(&format!("select-pane -t {}", sitat(&pane.id)), Svar::Ingenting);
        let (id, vindu) = (pane.id.clone(), pane.vindu_id.clone());
        self.peker = Some(id.clone());
        self.valgt = Some(pane);
        self.liste_følger = true;
        self.vis(vindu, Some(&id));
    }

    /// Henter panene i vinduet som vises, fra `alle`. `aktiv` overstyrer den
    /// aktive panen, for et svar fra tmux som kan være eldre enn
    /// `select-pane`.
    ///
    /// Er vinduet lukket, vises det tmux har gjort til gjeldende i sesjonen,
    /// eller vinduet til den valgte. Et vindu som ikke er listet ennå, er
    /// nytt og blir stående.
    pub(crate) fn oppdater_vindu(&mut self, aktiv: Option<&str>) {
        let lukket = self.vindu.first().filter(|p| {
            Some(&p.vindu_id) == self.vist.as_ref() && !self.alle.iter().any(|a| a.vindu_id == p.vindu_id)
        });
        if let Some(lukket) = lukket.cloned() {
            let gjeldende = self.alle.iter().find(|p| p.sesjon_id == lukket.sesjon_id && p.vindu_aktiv);
            self.vist = gjeldende.or(self.valgt.as_ref()).map(|p| p.vindu_id.clone());
        }
        let Some(vist) = self.vist.clone() else { return };
        let før = self.aktiv().map(|p| p.id.clone());
        self.vindu = self.alle.iter().filter(|p| p.vindu_id == vist).cloned().collect();
        if let Some(aktiv) = aktiv {
            for p in &mut self.vindu {
                p.aktiv = p.id == aktiv;
            }
        }
        if self.aktiv().map(|p| &p.id) != før.as_ref() {
            self.nullstill_aktiv();
        }
        self.utdatert = true;
    }

    /// Panene som vises i vinduet, og hvor. Med zoom er det bare én.
    pub(crate) fn bokser(&self) -> Vec<Boks> {
        let v = self.visning;
        if let Some(oppsett) = self.oppsett() {
            return bokser::plasser(&oppsett, v, self.utseende.paner);
        }
        let boks = |p: &Pane| Boks {
            pane: p.id.clone(),
            ramme: None,
            innhold: Rect::new(v.x + p.venstre, v.y + p.topp, p.bredde, p.høyde).intersection(v),
        };
        self.vindu.iter().map(boks).collect()
    }

    pub(crate) fn oppsett(&self) -> Option<oppsett::Celle> {
        oppsett::tolk(&self.vindu.first()?.oppsett)
    }

    /// Den aktive panen i vinduet, eller den valgte før vinduet dens er
    /// listet. Et nytt vindu har ingen før det er listet, så tastene ikke går
    /// til Claude-en i et annet vindu.
    pub(crate) fn aktiv(&self) -> Option<&Pane> {
        let valgt = self.valgt.as_ref().filter(|v| Some(&v.vindu_id) == self.vist.as_ref());
        self.vindu.iter().find(|p| p.aktiv).or(valgt)
    }

    pub(crate) fn aktiver(&mut self, id: &str) {
        self.send(&format!("select-pane -t {}", sitat(id)), Svar::Ingenting);
        if self.lister {
            self.aktivert = Some(id.to_string());
            self.list_igjen = true;
        }
        for p in &mut self.vindu {
            p.aktiv = p.id == id;
        }
        self.nullstill_aktiv();
    }

    pub(crate) fn nullstill_aktiv(&mut self) {
        self.aktiv_pane = AktivPane::default();
        self.utdatert = true;
    }

    /// Vinduet får størrelsen til visningen, minus rutene rammene trenger.
    ///
    /// Størrelsen settes for hvert vindu, ikke for klienten. Da får ikke et
    /// vindu du bytter til, størrelsen til det forrige før sin egen, og Claude
    /// Code tegner seg ikke på nytt to ganger. Klienten får den samme, for
    /// vinduer nerdr ikke har satt størrelse for ennå.
    ///
    /// Sendes bare når visningen endres eller vinduet deles på en annen måte,
    /// aldri fordi panene i tmux har endret størrelse. Et tmux-oppsett med en
    /// `client-resized`-hook som endrer panene, gir ellers en løkke.
    /// Med flere klienter i control mode får vinduet størrelsen til den
    /// minste, så en nerdr uten fokus har `ignore-size`, og den du går til,
    /// tar plassen. tmux regner størrelsen på nytt først når en klient ber om
    /// en, og derfor sendes den igjen.
    pub(crate) fn ta_plassen(&mut self) {
        self.send("refresh-client -f !ignore-size", Svar::Ingenting);
        self.størrelser.clear();
        self.synk_størrelse();
    }

    pub(crate) fn synk_størrelse(&mut self) {
        if let Some(vist) = self.vist.clone().filter(|_| !self.vindu.is_empty()) {
            self.synk_størrelse_for(&vist);
        }
    }

    pub(crate) fn synk_størrelse_for(&mut self, vindu: &str) {
        let v = self.visning;
        let Some(oppsett) = self.alle.iter().find(|p| p.vindu_id == vindu).and_then(|p| oppsett::tolk(&p.oppsett)) else {
            return;
        };
        let ekstra = bokser::ekstra(&oppsett, self.utseende.paner);
        if v.width <= ekstra.0 || v.height <= ekstra.1 {
            return;
        }
        let størrelse = (v.width - ekstra.0, v.height - ekstra.1);
        if self.størrelser.get(vindu) == Some(&størrelse) {
            return;
        }
        let (bredde, høyde) = størrelse;
        self.send(&format!("refresh-client -C {}", sitat(&format!("{vindu}:{bredde}x{høyde}"))), Svar::Ingenting);
        self.send(&format!("refresh-client -C {bredde}x{høyde}"), Svar::Ingenting);
        self.størrelser.insert(vindu.to_string(), størrelse);
        self.utdatert = true;
    }

    /// En sesjon nerdr har laget, hører til Claude-en, så den lukkes med det
    /// du har åpnet ved siden av, og det som fortsatt kjører, står i
    /// spørsmålet. I en sesjon du har laget selv, lukkes bare vinduet, og
    /// deler flere Claude-er vinduet, bare panen.
    pub(crate) fn lukk_sesjon(&mut self, pane: &Pane) {
        let andre = |p: &&Pane| p.id != pane.id && self.er_claude(&p.id);
        let flere_i_vinduet = self.alle.iter().filter(|p| p.vindu_id == pane.vindu_id).any(|p| andre(&p));
        let flere_i_sesjonen = self.alle.iter().filter(|p| p.sesjon_id == pane.sesjon_id).any(|p| andre(&p));
        let tittel = pane.oppgave().map_or_else(|| self.sted(pane), str::to_string);
        let programmer: Vec<&Pane> =
            self.alle.iter().filter(|p| p.sesjon_id == pane.sesjon_id && !self.er_claude(&p.id) && !p.bare_skall()).collect();
        let kjører: Vec<String> = programmer.iter().map(|p| format!("{} i vindu {}", p.kommando, p.vindu)).collect();
        let kjører = if kjører.is_empty() { String::new() } else { format!(", der {} fortsatt kjører", kjører.join(", ")) };
        let (hva, kommando) = match () {
            _ if flere_i_vinduet => {
                (format!("«{tittel}». De andre Claude-ene i vinduet står igjen."), format!("kill-pane -t {}", sitat(&pane.id)))
            }
            _ if matches!(self.agenter.get(&pane.id), Some(Agent::Rest { .. })) => {
                (format!("Sesjonen {}{kjører}.", pane.sesjon), format!("kill-session -t {}", sitat(&pane.sesjon_id)))
            }
            _ if pane.fra_nerdr && !flere_i_sesjonen => {
                (format!("«{tittel}» og sesjonen {}{kjører}.", pane.sesjon), format!("kill-session -t {}", sitat(&pane.sesjon_id)))
            }
            _ => (format!("«{tittel}» og vinduet den står i."), format!("kill-window -t {}", sitat(&pane.vindu_id))),
        };
        self.popup = Some(Popup::Lukke(Lukke { spørsmål: "lukke sesjonen?", hva, kommando }));
    }

    pub(crate) fn bytt_fokus(&mut self, fokus: Fokus) {
        if fokus == self.fokus {
            return;
        }
        self.låst = match fokus {
            Fokus::Pane => self.valgt.as_ref().and_then(|p| Some((p.id.clone(), self.plass(p)?))),
            Fokus::Liste => {
                self.forlatt = self.låst.take();
                None
            }
        };
        self.peker = self.valgt.as_ref().map(|p| p.id.clone());
        self.liste_følger = true;
        if fokus == Fokus::Liste {
            self.aktiv_pane.kopimodus = None;
        }
        // Det du har sett, er ledig, så det merkes først når du går ut. Ellers
        // ville den du går inn i, blitt ledig med en gang.
        if fokus == Fokus::Liste
            && let Some(id) = self.valgt.as_ref().map(|p| p.id.clone())
        {
            self.merk_sett(&id, nå_ms());
        }
        // Fra lista går du inn i panen du valgte, også når en annen er aktiv
        // i vinduet.
        let valgt = self.valgt.as_ref().map(|p| p.id.clone()).filter(|id| self.vindu.iter().any(|p| &p.id == id));
        if fokus == Fokus::Pane
            && let Some(id) = valgt.filter(|id| self.aktiv().is_some_and(|a| &a.id != id))
        {
            self.aktiver(&id);
        }
        self.fokus = fokus;
    }
}
