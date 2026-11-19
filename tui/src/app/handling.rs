//! Handlingene bak `:` og det som starter nye Claude-er.

use super::App;
use nerdr_kjerne::{start, tmux};
use nerdr_kjerne::kontroll::sitat;
use ratatui::crossterm::event::{KeyEvent, KeyModifiers};
use std::{env, thread};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;
use crate::{Fokus, Hendelse, Lukke, Svar, ny, utseende};
use crate::handlinger::{Handling, Handlinger};
use crate::popup::Popup;
use crate::utseende::Akse;

impl App {
    /// Samme kommandoer som `start::start`, men gjennom control mode, så svaret
    /// kommer som en melding.
    pub(crate) fn start_claude(&mut self, mappe: PathBuf, samtale: Option<&str>) {
        let navn = start::sesjonsnavn(&mappe);
        let claude_kommando = start::claude(start::Valg { samtale, ..start::Valg::default() });
        self.send(&tmux::linje(&start::ny_sesjon(&navn, &mappe)), Svar::NySesjon { navn, forsøk: 1, mappe, claude_kommando });
    }

    pub(crate) fn velg_treff(&mut self) {
        match &self.popup {
            Some(Popup::Handlinger(_)) => self.kjør_valgt_handling(),
            Some(Popup::Velger(_)) => self.start_valgt(),
            _ => {}
        }
    }

    pub(crate) fn kjør_valgt_handling(&mut self) {
        let Some(Popup::Handlinger(h)) = self.popup.take() else { return };
        if let Some(handling) = h.valgt() {
            self.kjør(handling);
        }
    }

    /// Uten treff står velgeren åpen, så du kan søke på nytt.
    pub(crate) fn start_valgt(&mut self) {
        let Some(Popup::Velger(v)) = &self.popup else { return };
        let Some((mappe, samtale)) = v.valgt().map(|k| (k.mappe.clone(), k.samtale.clone())) else { return };
        self.popup = None;
        self.start_claude(mappe, samtale.as_deref());
    }

    pub(crate) fn lukke(&self) -> Option<&Lukke> {
        match &self.popup {
            Some(Popup::Lukke(l)) => Some(l),
            _ => None,
        }
    }

    pub(crate) fn åpne_handlinger(&mut self) {
        self.popup = Some(Popup::Handlinger(Handlinger::ny(&self.prefiks.navn(), &self.bindinger, &self.utseende)));
    }

    pub(crate) fn kjør(&mut self, handling: Handling) {
        match handling {
            Handling::Prefiks(kode, ctrl) => {
                let m = if ctrl { KeyModifiers::CONTROL } else { KeyModifiers::NONE };
                self.prefikstast(KeyEvent::new(kode, m));
            }
            Handling::Utseende(akse, i) => {
                self.utseende.sett(akse, i);
                self.lagre_utseende(akse);
            }
        }
    }

    /// Bredden står i en egen option, sammen med stripa, og overstyrer den
    /// du har dratt til.
    pub(crate) fn lagre_utseende(&mut self, akse: Akse) {
        if akse == Akse::Bredde {
            self.liste_bredde = self.utseende.bredde.kolonner();
            self.liste_stripe = false;
            self.lagre_liste();
        }
        // Rammene endrer hvor mye plass tmux får.
        self.utdatert = true;
        self.send(&format!("set-option -g {} {}", utseende::OPTION, sitat(&self.utseende.tekst())), Svar::Ingenting);
    }

    /// Det som starter en Claude, går via lista, så du går inn i den nye selv
    /// om du startet den fra en pane.
    pub(crate) fn ny_sesjon(&mut self, fortsett: bool) {
        self.bytt_fokus(Fokus::Liste);
        self.åpne_velger(fortsett);
    }

    pub(crate) fn ny_chat(&mut self) {
        match start::chatmappe() {
            Ok(mappe) => {
                self.bytt_fokus(Fokus::Liste);
                self.start_claude(mappe, None);
            }
            Err(feil) => self.beskjed = Some((format!("fikk ikke laget mappa for chat: {feil}"), Instant::now())),
        }
    }

    /// Det tar sekunder å lese alle mappene i `$HOME`, så velgeren åpner med
    /// en gang og får mappene mens de leses. De leses på nytt hver gang, så en
    /// mappe du nettopp har laget, er med.
    pub(crate) fn åpne_velger(&mut self, fortsett: bool) {
        let kjører: HashSet<String> = self.agenter.values().filter_map(|a| a.claude()?.samtale.clone()).collect();
        let velger = ny::Velger::ny(fortsett, &self.konfig.join("projects"), &kjører);
        let (røtter, stopp, tx) = (velger.røtter(), velger.stopp(), self.tx.clone());
        let hjem = PathBuf::from(env::var("HOME").unwrap_or_default());
        thread::spawn(move || {
            ny::les_mapper(&røtter, &hjem, &stopp, |mapper, ferdig| {
                let _ = tx.send(Hendelse::Mapper(stopp.clone(), mapper, ferdig));
            });
        });
        self.popup = Some(Popup::Velger(velger));
    }
}
