//! Tastene, også i kopimodus.

use super::App;
use nerdr_kjerne::kontroll::{dobbelt_sitat, sitat};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;
use crate::{Fokus, Svar, bindinger, taster, tegn};
use crate::kopimodus::{Innhold, Kopimodus, Utfall};
use crate::markering::Markering;
use crate::popup::{Popup, Søketast};
use crate::utseende::Akse;

impl App {
    /// Tasten etter prefikset, med bindingene fra tmux-oppsettet ditt.
    pub(crate) fn etter_prefikset(&mut self, k: KeyEvent) {
        match self.bindinger.finn(&k).cloned() {
            _ if self.prefiks.er(&k) => self.prefikstast(k),
            Some(hva) => self.kjør_binding(hva, &format!("{} ", self.prefiks.navn())),
            None => self.prefikstast(k),
        }
        if self.bindinger.gjentas(&k) {
            self.gjentar = Some(Instant::now() + self.bindinger.gjentatid);
        }
    }

    /// `foran` står foran tasten i beskjeden, som prefikset.
    pub(crate) fn kjør_binding(&mut self, hva: bindinger::Hva, foran: &str) {
        match hva {
            bindinger::Hva::Som(nerdr) => self.prefikstast(nerdr),
            bindinger::Hva::Send(kommandoer) => {
                // Vinduet, så tmux velger panen, som i `prefikstast`.
                let mål = self.vist.clone().or_else(|| self.aktiv().map(|p| p.id.clone()));
                self.ut.for_seg(mål.as_deref(), &kommandoer);
            }
            bindinger::Hva::Trenger { tast, hva } => {
                let tekst = format!("{foran}{tast} åpner {hva} i tmux, som nerdr ikke kan vise");
                self.beskjed = Some((tekst, Instant::now()));
            }
        }
    }

    /// Tasten nerdr har etter prefikset, som er den samme som i tmux uten
    /// noe oppsett. Virker både fra lista og fra panen.
    pub(crate) fn prefikstast(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let alt = k.modifiers.contains(KeyModifiers::ALT);
        match k.code {
            KeyCode::Char('w' | 's') if !ctrl => return self.bytt_fokus(Fokus::Liste),
            KeyCode::Char(')') => return self.bla(1),
            KeyCode::Char('(') => return self.bla(-1),
            KeyCode::Char('n') if !ctrl => return self.neste_vindu(1),
            KeyCode::Char('p') if !ctrl => return self.neste_vindu(-1),
            KeyCode::Char('l') if !ctrl => {
                if let Some(forrige) = self.forrige_vindu.clone() {
                    self.vis_vindu(forrige);
                }
                return;
            }
            KeyCode::Char(c @ '0'..='9') => {
                let fane = self.faner().into_iter().find(|p| p.vindu == c.to_string()).map(|p| p.vindu_id.clone());
                if let Some(vindu) = fane {
                    self.vis_vindu(vindu);
                }
                return;
            }
            KeyCode::Char('d') if !ctrl => return self.ferdig = Some(String::new()),
            KeyCode::Char('b') if !ctrl => return self.bytt_stripe(),
            KeyCode::Char('u') if !ctrl => {
                self.popup = Some(Popup::Utseende(Akse::Plass));
                return;
            }
            KeyCode::Char('?') => return self.popup = Some(Popup::Hjelp(0)),
            KeyCode::Char(c @ ('f' | 'r')) if !ctrl => return self.ny_sesjon(c == 'r'),
            KeyCode::Char('F') if !ctrl => return self.ny_chat(),
            KeyCode::Char(':') => return self.åpne_handlinger(),
            KeyCode::Char('[') => return self.start_kopimodus(),
            _ => {}
        }
        let (Some(aktiv), Some(vindu)) = (self.aktiv().cloned(), self.vist.as_ref().map(|v| sitat(v))) else {
            return;
        };
        let id = sitat(&aktiv.id);
        // Til vinduet, så tmux velger panen: den nerdr har, er eldre rett etter
        // en split, og `x` lukket da Claude-en.
        let retning = match k.code {
            KeyCode::Left => Some("-L"),
            KeyCode::Right => Some("-R"),
            KeyCode::Up => Some("-U"),
            KeyCode::Down => Some("-D"),
            _ => None,
        };
        let kommando = match (k.code, retning) {
            _ if self.prefiks.er(&k) => {
                let Some(args) = taster::send_keys(&self.prefiks.tast()) else { return };
                format!("send-keys -t {id} {args}")
            }
            (_, Some(retning)) if ctrl || alt => format!("resize-pane {retning} -t {vindu} {}", if alt { 5 } else { 1 }),
            (_, Some(retning)) => format!("select-pane {retning} -t {vindu}"),
            (KeyCode::Char('%'), _) => format!("split-window -h -t {vindu} -c '#{{pane_current_path}}'"),
            (KeyCode::Char('"'), _) => format!("split-window -v -t {vindu} -c '#{{pane_current_path}}'"),
            (KeyCode::Char('o'), _) if ctrl => format!("rotate-window -t {vindu}"),
            (KeyCode::Char('o'), _) => {
                let i = self.vindu.iter().position(|p| p.aktiv).unwrap_or(0);
                if let Some(neste) = self.vindu.get((i + 1) % self.vindu.len().max(1)).map(|p| p.id.clone()) {
                    self.aktiver(&neste);
                }
                return;
            }
            (KeyCode::Char(';'), _) => format!("last-pane -t {vindu}"),
            (KeyCode::Char('z'), _) => format!("resize-pane -Z -t {vindu}"),
            (KeyCode::Char('{'), _) => format!("swap-pane -U -t {vindu}"),
            (KeyCode::Char('}'), _) => format!("swap-pane -D -t {vindu}"),
            (KeyCode::Char(' '), _) => format!("next-layout -t {vindu}"),
            (KeyCode::Char('c'), _) => {
                let sesjon = sitat(&format!("{}:", aktiv.sesjon_id));
                self.send(&format!("new-window -t {sesjon} -c '#{{pane_current_path}}' -P -F '#{{window_id}}'"), Svar::NyttVindu);
                return;
            }
            (KeyCode::Char('x'), _) => format!("kill-pane -t {vindu}"),
            (KeyCode::Char('&'), _) => format!("kill-window -t {}", sitat(&aktiv.vindu_id)),
            (KeyCode::PageUp, _) => {
                let side = self.bokser().iter().find(|b| b.pane == aktiv.id).map_or(1, |b| b.innhold.height.max(1));
                return self.rull_til(self.aktiv_pane.rull.saturating_add(side as u32));
            }
            _ => return,
        };
        self.send(&kommando, Svar::Ingenting);
    }

    pub(crate) fn tast(&mut self, k: KeyEvent) -> bool {
        match &self.popup {
            None => {}
            Some(Popup::Hjelp(_)) => return self.hjelp_tast(k),
            Some(Popup::Utseende(akse)) => return self.utseende_tast(*akse, k),
            Some(Popup::Lukke(_)) => {
                if let Some(Popup::Lukke(lukke)) = self.popup.take()
                    && k.code == KeyCode::Char('y')
                {
                    self.send(&lukke.kommando, Svar::Ingenting);
                }
                return true;
            }
            Some(Popup::Handlinger(_) | Popup::Velger(_)) => return self.søk_tast(k),
        }
        if let Some(til) = self.gjentar.take()
            && Instant::now() < til
            && self.bindinger.gjentas(&k)
        {
            self.etter_prefikset(k);
            return true;
        }
        if std::mem::take(&mut self.etter_prefiks) {
            self.etter_prefikset(k);
            return true;
        }
        if self.prefiks.er(&k) {
            self.etter_prefiks = true;
            return true;
        }
        if self.fokus == Fokus::Pane && self.aktiv_pane.kopimodus.is_some() {
            return self.kopitast(k);
        }
        if self.fokus == Fokus::Pane {
            if let Some(hva) = self.root.finn(&k).cloned() {
                self.kjør_binding(hva, "");
                return true;
            }
            if let (Some(pane), Some(args)) = (self.aktiv(), taster::send_keys(&k)) {
                let kommando = format!("send-keys -t {} {args}", sitat(&pane.id));
                self.send(&kommando, Svar::Ingenting);
                self.rull_til(0);
            }
            return self.aktiv_pane.markering.take().is_some();
        }
        // Bokstaver i lista gjør ingenting utenom h, j, k og l, som pilene, så
        // den som ikke er vant til moduser, ikke trykker på noe uten å vite det.
        match k.code {
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                self.ferdig = Some(String::new())
            }
            KeyCode::Up | KeyCode::Char('k') => self.flytt(-1),
            KeyCode::Down | KeyCode::Char('j') => self.flytt(1),
            KeyCode::Left | KeyCode::Char('h') => return self.brett(true),
            KeyCode::Right | KeyCode::Char('l') => return self.brett(false),
            KeyCode::Enter if self.valgt_prosjekt().is_some_and(|p| self.brettet.contains(&p)) => return self.brett(false),
            KeyCode::Enter => self.gå_inn(),
            KeyCode::Esc if self.valgt.is_some() => self.bytt_fokus(Fokus::Pane),
            _ => return false,
        }
        true
    }

    pub(crate) fn hjelp_tast(&mut self, k: KeyEvent) -> bool {
        let steg = match k.code {
            KeyCode::Up | KeyCode::Char('k') => -1,
            KeyCode::Down | KeyCode::Char('j') => 1,
            _ => 0,
        };
        // Får hjelpen plass, lukker hver tast den, som hintet sier.
        if steg == 0 || tegn::hjelp_rull_maks(self) == 0 {
            self.popup = None;
        } else {
            self.rull_hjelp(steg);
        }
        true
    }

    pub(crate) fn utseende_tast(&mut self, akse: Akse, k: KeyEvent) -> bool {
        let steg = match k.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                self.popup = None;
                return true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.popup = Some(Popup::Utseende(self.utseende.nabo(akse, -1)));
                return true;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.popup = Some(Popup::Utseende(self.utseende.nabo(akse, 1)));
                return true;
            }
            KeyCode::Left | KeyCode::Char('h') => -1,
            KeyCode::Right | KeyCode::Char('l' | ' ') => 1,
            _ => return false,
        };
        self.utseende.endre(akse, steg);
        self.lagre_utseende(akse);
        true
    }

    pub(crate) fn søk_tast(&mut self, k: KeyEvent) -> bool {
        let side = tegn::menyrader(self) as isize;
        let Some(popup) = &mut self.popup else { return false };
        match popup.søketast(k, side) {
            Søketast::Lukk => self.popup = None,
            Søketast::Velg => self.velg_treff(),
            Søketast::Ubrukt => return false,
            Søketast::Bytt | Søketast::Endret | Søketast::Flyttet => {}
        }
        true
    }

    pub(crate) fn flytt_i_menyen(&mut self, steg: isize) {
        if let Some(popup) = &mut self.popup {
            popup.flytt(steg);
        }
    }

    /// `paste-buffer -p` legger på markørene for bracketed paste bare når panen
    /// har bedt om dem, og uten dem sender Claude inn ved første linjeskift.
    pub(crate) fn lim_inn(&mut self, tekst: &str) {
        let Some(pane) = self.aktiv().filter(|_| self.fokus == Fokus::Pane) else { return };
        let id = sitat(&pane.id);
        // paste-buffer gjør LF om til CR, så CRLF ville blitt to linjeskift.
        let tekst = tekst.replace("\r\n", "\n");
        self.send(&format!("set-buffer -b nerdr {}", dobbelt_sitat(&tekst)), Svar::Ingenting);
        self.send(&format!("paste-buffer -p -d -b nerdr -t {id}"), Svar::Ingenting);
        self.rull_til(0);
    }

    /// Som `prefix [` i tmux. Fra lista går du inn i panen først.
    pub(crate) fn start_kopimodus(&mut self) {
        self.bytt_fokus(Fokus::Pane);
        let (Some(innhold), Some(pane)) = (self.innhold(), self.aktiv().map(|p| p.id.clone())) else { return };
        self.aktiv_pane.markering = None;
        self.kopiert = None;
        self.aktiv_pane.kopimodus = Some(Kopimodus::ny(pane, &innhold, self.aktiv_pane.markør));
    }

    pub(crate) fn innhold(&self) -> Option<Innhold> {
        let pane = self.aktiv()?;
        let bilde = self.bilder.get(&pane.id).map_or(&[][..], Vec::as_slice);
        Some(Innhold::ny(bilde, self.aktiv_pane.historie, self.aktiv_pane.rull, pane.bredde, pane.høyde))
    }

    pub(crate) fn kopitast(&mut self, k: KeyEvent) -> bool {
        let (Some(innhold), Some(modus)) = (self.innhold(), self.aktiv_pane.kopimodus.as_mut()) else { return false };
        match modus.tast(k, &innhold) {
            Utfall::Rull(rull) => self.rull_til(rull),
            Utfall::Kopier(markering) => {
                self.kopier(markering);
                self.avslutt_kopimodus();
            }
            Utfall::Ut => self.avslutt_kopimodus(),
            Utfall::Ingenting => return false,
        }
        true
    }

    /// Linjene hentes på nytt, fordi bildet bare har det som står på
    /// skjermen, og markeringen kan være lengre.
    pub(crate) fn kopier(&mut self, markering: Markering) {
        let Some(pane) = self.aktiv() else { return };
        let (id, bredde) = (sitat(&pane.id), pane.bredde);
        let (fra, til) = markering.spenn();
        let fra = fra.max(0);
        let historie = self.aktiv_pane.historie as i64;
        let kommando = format!("capture-pane -p -N -t {id} -S {} -E {}", fra - historie, til - historie);
        self.send(&kommando, Svar::Kopi { markering, fra, bredde });
    }

    /// Som i tmux står panen nederst igjen etterpå.
    pub(crate) fn avslutt_kopimodus(&mut self) {
        self.aktiv_pane.kopimodus = None;
        self.rull_til(0);
    }

    /// Den du lager i kopimodus, eller den du drar med musen.
    pub(crate) fn vist_markering(&self) -> Option<&Markering> {
        self.aktiv_pane.kopimodus.as_ref().and_then(|k| k.markering.as_ref()).or(self.aktiv_pane.markering.as_ref())
    }
}
