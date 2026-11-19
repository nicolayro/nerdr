//! Musen, og det den drar og ruller.

use super::{App, LISTE_OPTION};
use nerdr_kjerne::kontroll::sitat;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::time::{Duration, Instant};
use crate::{Fokus, Svar, taster, tegn};
use crate::markering::{Markering, Punkt};
use crate::popup::Popup;
use crate::tegn::Mål;
use crate::utseende::Akse;

/// Linjer per hakk på hjulet, som piltaster i alternativ skjerm og som linjer
/// i historikken.
const RULL_STEG: u32 = 3;
/// Terminalen sender to klikk og sier ikke fra om dobbeltklikk.
const DOBBELTKLIKK: Duration = Duration::from_millis(500);

impl App {
    pub(crate) fn rull_til(&mut self, rull: u32) {
        let rull = rull.min(self.aktiv_pane.historie);
        if rull != self.aktiv_pane.rull {
            self.aktiv_pane.rull = rull;
            self.utdatert = true;
        }
    }

    pub(crate) fn rull_hjelp(&mut self, steg: i16) {
        let maks = tegn::hjelp_rull_maks(self);
        if let Some(Popup::Hjelp(rull)) = &mut self.popup {
            *rull = rull.saturating_add_signed(steg).min(maks);
        }
    }

    pub(crate) fn mus(&mut self, m: MouseEvent) -> bool {
        let før = self.under_musen();
        self.musen = Some((m.column, m.row));
        let flyttet = self.under_musen() != før;
        // Et nytt klikk fjerner markeringen, også når det starter en ny, og
        // avslutter kopimodus.
        let klikk = matches!(m.kind, MouseEventKind::Down(_));
        let fjernet = klikk && (self.aktiv_pane.markering.take().is_some() || self.aktiv_pane.kopimodus.take().is_some());
        self.håndter_mus(m) || flyttet || fjernet
    }

    /// Regnes ut på nytt hver gang, så det følger med når lista flytter seg
    /// under en mus som står stille.
    pub(crate) fn under_musen(&self) -> Option<Mål> {
        let (x, y) = self.musen?;
        tegn::treff::mål(self, x, y)
    }

    pub(crate) fn håndter_mus(&mut self, m: MouseEvent) -> bool {
        if self.drar_skille {
            match m.kind {
                MouseEventKind::Drag(MouseButton::Left) => self.dra_skille(m.column),
                MouseEventKind::Up(MouseButton::Left) => {
                    self.drar_skille = false;
                    self.lagre_liste();
                }
                _ => return false,
            }
            return true;
        }
        // Slippet går bare til panen når klikket gjorde det, så et klikk i
        // lista ikke gir et slipp i Claude-en.
        if m.kind == MouseEventKind::Up(MouseButton::Left) && std::mem::take(&mut self.svelg_slipp) {
            return false;
        }
        if m.kind == MouseEventKind::Down(MouseButton::Left) {
            self.svelg_slipp = true;
        }
        let mål = tegn::treff::mål(self, m.column, m.row);
        if self.popup.is_some() {
            return self.mus_i_popupen(m, mål);
        }
        if self.aktiv_pane.markering.as_ref().is_some_and(|m| m.drar)
            && matches!(m.kind, MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left))
        {
            return self.dra_markering(m);
        }
        let Some(mål) = mål else { return false };
        let venstre = m.kind == MouseEventKind::Down(MouseButton::Left);
        if self.visning.x > m.column && mål != Mål::Skille {
            match m.kind {
                MouseEventKind::ScrollUp => return self.rull_lista(-(RULL_STEG as i32)),
                MouseEventKind::ScrollDown => return self.rull_lista(RULL_STEG as i32),
                _ => {}
            }
        }
        // Målet er funnet før noe endres, så kortene ikke flytter seg under klikket.
        match mål {
            Mål::Pane(_) | Mål::Visning => self.mus_i_panen(m),
            Mål::Fane(vindu) | Mål::LukkFane(vindu) if m.kind == MouseEventKind::Down(MouseButton::Middle) => {
                self.send(&format!("kill-window -t {}", sitat(&vindu)), Svar::Ingenting);
                true
            }
            _ if !venstre => false,
            Mål::Skille => {
                let dobbelt = self.sist_klikket.take().is_some_and(|(mål, t)| mål == Mål::Skille && t.elapsed() < DOBBELTKLIKK);
                if dobbelt {
                    self.bytt_stripe();
                } else {
                    self.sist_klikket = Some((Mål::Skille, Instant::now()));
                    self.drar_skille = true;
                }
                true
            }
            Mål::Knapp(knapp) => {
                self.prefikstast(KeyEvent::new(KeyCode::Char(tegn::tasten_til(knapp)), KeyModifiers::NONE));
                true
            }
            Mål::LukkFane(vindu) => {
                self.send(&format!("kill-window -t {}", sitat(&vindu)), Svar::Ingenting);
                true
            }
            Mål::Fane(vindu) => {
                self.vis_vindu(vindu);
                true
            }
            Mål::NyFane => {
                self.prefikstast(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
                true
            }
            Mål::Sortering => {
                self.utseende.endre(Akse::Orden, 1);
                self.lagre_utseende(Akse::Orden);
                true
            }
            Mål::Pil(opp) => {
                let side = tegn::utsnitt(self).0.rom.max(1) as i32;
                self.rull_lista(if opp { -side } else { side })
            }
            Mål::Lukk(id) => {
                if let Some(pane) = self.alle.iter().find(|p| p.id == id).cloned() {
                    self.lukk_sesjon(&pane);
                }
                true
            }
            Mål::Prosjekt(navn) => {
                if !self.brettet.remove(&navn) {
                    self.brettet.insert(navn);
                }
                true
            }
            Mål::Kort(id) => {
                // Via lista, så panen du forlater, merkes som sett før du går inn.
                self.bytt_fokus(Fokus::Liste);
                self.peker = Some(id);
                self.gå_inn();
                true
            }
            Mål::Liste => {
                let ute = self.fokus == Fokus::Pane;
                self.bytt_fokus(Fokus::Liste);
                ute
            }
            Mål::Svar(_) | Mål::Rad(_) | Mål::Verdi(..) | Mål::Popup => false,
        }
    }

    /// Lista ruller bare når kortene ikke får plass.
    pub(crate) fn rull_lista(&mut self, steg: i32) -> bool {
        if !tegn::utsnitt(self).0.ruller {
            return false;
        }
        // Tegningen holder rullet innenfor lista.
        self.liste_rull = (self.liste_rull as i32 + steg).max(0) as u16;
        self.liste_følger = false;
        true
    }

    pub(crate) fn mus_i_popupen(&mut self, m: MouseEvent, mål: Option<Mål>) -> bool {
        let venstre = m.kind == MouseEventKind::Down(MouseButton::Left);
        match &self.popup {
            Some(Popup::Hjelp(_)) => {
                match m.kind {
                    MouseEventKind::ScrollUp => self.rull_hjelp(-(RULL_STEG as i16)),
                    MouseEventKind::ScrollDown => self.rull_hjelp(RULL_STEG as i16),
                    _ => return false,
                }
            }
            Some(Popup::Handlinger(_) | Popup::Velger(_)) => match m.kind {
                MouseEventKind::ScrollUp => self.flytt_i_menyen(-1),
                MouseEventKind::ScrollDown => self.flytt_i_menyen(1),
                // Et klikk utenfor lukker, som Esc.
                MouseEventKind::Down(MouseButton::Left) => match mål {
                    Some(Mål::Rad(i)) => {
                        if let Some(felt) = self.popup.as_mut().and_then(|p| p.felt()) {
                            felt.valgt = i;
                        }
                        self.velg_treff();
                    }
                    Some(_) => {}
                    None => self.popup = None,
                },
                _ => return false,
            },
            Some(Popup::Utseende(_)) => {
                if !venstre {
                    return false;
                }
                match mål {
                    Some(Mål::Verdi(akse, i)) => {
                        self.popup = Some(Popup::Utseende(akse));
                        self.utseende.sett(akse, i);
                        self.lagre_utseende(akse);
                    }
                    Some(_) => {}
                    None => self.popup = None,
                }
            }
            Some(Popup::Lukke(_)) => {
                if !venstre {
                    return false;
                }
                // Et klikk utenfor popupen avbryter, som en tast som ikke er y.
                match mål {
                    Some(Mål::Svar(true)) => {
                        if let Some(Popup::Lukke(lukke)) = self.popup.take() {
                            self.send(&lukke.kommando, Svar::Ingenting);
                        }
                    }
                    Some(Mål::Popup) => {}
                    _ => self.popup = None,
                }
            }
            None => return false,
        }
        true
    }

    pub(crate) fn mus_i_panen(&mut self, m: MouseEvent) -> bool {
        let byttet = matches!(m.kind, MouseEventKind::Down(_)) && self.fokus != Fokus::Pane;
        if byttet {
            self.bytt_fokus(Fokus::Pane);
        }
        let inni = |r: Rect| (r.left()..r.right()).contains(&m.column) && (r.top()..r.bottom()).contains(&m.row);
        let bokser = self.bokser();
        let Some(boks) = bokser.iter().find(|b| inni(b.ramme.unwrap_or(b.innhold))) else { return byttet };
        let Some(pane) = self.vindu.iter().find(|p| p.id == boks.pane).cloned() else { return byttet };
        if !pane.aktiv {
            if matches!(m.kind, MouseEventKind::Down(_)) {
                self.aktiver(&pane.id);
                return true;
            }
            return byttet;
        }
        if !inni(boks.innhold) {
            return byttet;
        }
        // Klikket som tar deg inn, går ikke videre, så det ikke trykker på et
        // valg i en dialog.
        if byttet {
            return true;
        }
        self.svelg_slipp = false;
        let (id, x, y) = (sitat(&pane.id), m.column - boks.innhold.x, m.row - boks.innhold.y);
        if !self.aktiv_pane.mus && m.kind == MouseEventKind::Down(MouseButton::Left) {
            self.aktiv_pane.markering = Some(Markering::ny(pane.id, self.punkt(x, y)));
            return true;
        }
        let opp = match m.kind {
            MouseEventKind::ScrollUp => Some(true),
            MouseEventKind::ScrollDown => Some(false),
            _ => None,
        };
        match opp {
            _ if m.kind == MouseEventKind::Moved && !self.aktiv_pane.bevegelse => {}
            _ if self.aktiv_pane.mus => {
                if let Some(args) = taster::mus(&m, x, y) {
                    self.send(&format!("send-keys -t {id} {args}"), Svar::Ingenting);
                }
            }
            Some(opp) if self.aktiv_pane.alternativ => {
                let tast = if opp { "Up" } else { "Down" };
                self.send(&format!("send-keys -t {id} -N {RULL_STEG} {tast}"), Svar::Ingenting);
            }
            Some(true) => self.rull_til(self.aktiv_pane.rull.saturating_add(RULL_STEG)),
            Some(false) => self.rull_til(self.aktiv_pane.rull.saturating_sub(RULL_STEG)),
            None => {}
        }
        byttet
    }

    /// Markeringen følger musen også utenfor panen, men stopper ved kanten.
    pub(crate) fn dra_markering(&mut self, m: MouseEvent) -> bool {
        let pane = self.aktiv_pane.markering.as_ref().map(|m| m.pane.clone());
        let Some(boks) = self.bokser().into_iter().find(|b| Some(&b.pane) == pane.as_ref()) else {
            self.aktiv_pane.markering = None;
            return true;
        };
        let i = boks.innhold;
        let x = m.column.clamp(i.x, i.right().saturating_sub(1)) - i.x;
        let y = m.row.clamp(i.y, i.bottom().saturating_sub(1)) - i.y;
        let punkt = self.punkt(x, y);
        let Some(markering) = &mut self.aktiv_pane.markering else { return false };
        markering.dra(punkt);
        if m.kind != MouseEventKind::Up(MouseButton::Left) {
            return true;
        }
        markering.drar = false;
        if markering.tom() {
            self.aktiv_pane.markering = None;
            return true;
        }
        let topp = self.punkt(0, 0).0;
        if let (Some(markering), Some(bilde)) = (&self.aktiv_pane.markering, self.bilder.get(&boks.pane))
            && self.ut.kopier(&markering.tekst(bilde, topp, i.width))
        {
            self.kopiert = Some(Instant::now());
        }
        true
    }

    /// Streken følger musen. Drar du den forbi `LISTE_MIN`, blir lista en
    /// stripe, og drar du den ut igjen, får den bredden der musen er.
    pub(crate) fn dra_skille(&mut self, x: u16) {
        let bredde = x + 1;
        if bredde != self.visning.x {
            // Et klikk på streken etter å ha dratt den, er ikke et dobbeltklikk.
            self.sist_klikket = None;
        }
        self.liste_stripe = bredde < (tegn::stripebredde(self.utseende.paner) + tegn::LISTE_MIN) / 2;
        if !self.liste_stripe {
            self.liste_bredde = bredde.max(tegn::LISTE_MIN).min(self.skjerm.width / 2);
        }
    }

    pub(crate) fn bytt_stripe(&mut self) {
        self.liste_stripe = !self.liste_stripe;
        self.lagre_liste();
    }

    pub(crate) fn lagre_liste(&mut self) {
        let verdi = format!("{}{}", self.liste_bredde, if self.liste_stripe { " stripe" } else { "" });
        self.send(&format!("set-option -g {LISTE_OPTION} {}", sitat(&verdi)), Svar::Ingenting);
    }

    /// Der `x` og `y` i den aktive panen er, med linja regnet fra toppen av
    /// historikken.
    pub(crate) fn punkt(&self, x: u16, y: u16) -> Punkt {
        (self.aktiv_pane.historie as i64 - self.aktiv_pane.rull as i64 + y as i64, x)
    }
}
