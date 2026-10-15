mod taster;
mod tegn;

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};
use std::{env, io, thread};

use nerdr_kjerne::kontroll::{Kontroll, Melding, sitat, synkronisert_etter};
use nerdr_kjerne::paner::{FELT, Pane};
use nerdr_kjerne::sgr::{self, Linje};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;

const LISTE_INTERVALL: Duration = Duration::from_secs(2);
/// Øvre grense for hvor ofte panen hentes på nytt, omtrent 30 bilder i sekundet.
const HENTE_INTERVALL: Duration = Duration::from_millis(33);
/// Så lenge venter vi på at en synkronisert oppdatering blir ferdig, i tilfelle
/// avslutningen aldri kommer.
const SYNK_MAKS: Duration = Duration::from_millis(200);

enum Hendelse {
    Tmux(Melding),
    Term(Event),
}

/// Hva neste svar fra tmux gjelder. Svarene kommer i samme rekkefølge som
/// kommandoene, så køen holder rede på dem.
enum Venter {
    Paner,
    Bilde,
    Markør,
    Ingenting,
}

#[derive(Clone, Copy, PartialEq)]
enum Fokus {
    Liste,
    Pane,
}

struct App {
    kontroll: Kontroll,
    kø: VecDeque<Venter>,

    paner: Vec<Pane>,
    lister: bool,
    sist_listet: Option<Instant>,

    /// Panen som er valgt i lista og vises til høyre.
    valgt: Option<Pane>,
    bilde: Vec<Linje>,
    markør: Option<(u16, u16)>,
    utdatert: bool,
    henter: bool,
    sist_hentet: Instant,
    /// Den viste panen er midt i en synkronisert oppdatering.
    synk: bool,
    synk_siden: Instant,
    /// Slutten av forrige `%output`, i tilfelle en synkmarkør er delt over to.
    output_hale: String,
    /// Størrelsen sist sendt med `refresh-client -C`.
    størrelse: Option<(u16, u16)>,

    fokus: Fokus,
    visning: Rect,
    ferdig: Option<String>,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let socket = match args.as_slice() {
        [] => None,
        [flagg, socket] if flagg == "-L" => Some(socket.as_str()),
        _ => {
            eprintln!("bruk: nerdr [-L socket]");
            std::process::exit(2);
        }
    };
    if let Err(feil) = kjør(socket) {
        eprintln!("nerdr: {feil}");
        std::process::exit(1);
    }
}

fn kjør(socket: Option<&str>) -> io::Result<()> {
    let (tx, rx) = mpsc::channel();
    let tmux_tx = tx.clone();
    let kontroll = Kontroll::start(socket, move |m| {
        let _ = tmux_tx.send(Hendelse::Tmux(m));
    })?;
    thread::spawn(move || {
        while let Ok(e) = event::read() {
            if tx.send(Hendelse::Term(e)).is_err() {
                break;
            }
        }
    });

    let mut terminal = ratatui::init();
    let mut app = App::ny(kontroll);
    let res = app.løkke(&mut terminal, &rx);
    ratatui::restore();

    res?;
    match app.ferdig {
        Some(grunn) if !grunn.is_empty() => Err(io::Error::other(grunn)),
        _ => Ok(()),
    }
}

impl App {
    fn ny(kontroll: Kontroll) -> App {
        App {
            kontroll,
            kø: VecDeque::new(),
            paner: Vec::new(),
            lister: false,
            sist_listet: None,
            valgt: None,
            bilde: Vec::new(),
            markør: None,
            utdatert: false,
            henter: false,
            sist_hentet: Instant::now(),
            synk: false,
            synk_siden: Instant::now(),
            output_hale: String::new(),
            størrelse: None,
            fokus: Fokus::Liste,
            visning: Rect::default(),
            ferdig: None,
        }
    }

    fn løkke(&mut self, terminal: &mut DefaultTerminal, rx: &Receiver<Hendelse>) -> io::Result<()> {
        self.list_paner();
        let mut tegne = true;
        while self.ferdig.is_none() {
            if tegne {
                self.visning = tegn::visning(terminal.size()?);
                terminal.draw(|f| tegn::alt(f, self))?;
                self.synk_størrelse();
                tegne = false;
            }

            let første = match rx.recv_timeout(HENTE_INTERVALL) {
                Ok(h) => Some(h),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            };
            // Alt som har hopet seg opp, tas før neste tegning, så en strøm av
            // %output gir én henting og ikke én per linje.
            for h in første.into_iter().chain(rx.try_iter()) {
                tegne |= self.håndter(h);
            }

            if self.sist_listet.is_some_and(|t| t.elapsed() >= LISTE_INTERVALL) {
                self.list_paner();
            }
            let ferdig_tegnet = !self.synk || self.synk_siden.elapsed() >= SYNK_MAKS;
            if self.utdatert
                && ferdig_tegnet
                && !self.henter
                && self.sist_hentet.elapsed() >= HENTE_INTERVALL
            {
                self.hent();
            }
        }
        Ok(())
    }

    fn send(&mut self, kommando: &str, venter: Venter) {
        match self.kontroll.send(kommando) {
            Ok(()) => self.kø.push_back(venter),
            Err(feil) => self.ferdig = Some(format!("tmux: {feil}")),
        }
    }

    fn håndter(&mut self, h: Hendelse) -> bool {
        match h {
            Hendelse::Tmux(Melding::Svar { ok, linjer }) => match self.kø.pop_front() {
                Some(venter) => self.svar(venter, ok, linjer),
                None => false,
            },
            Hendelse::Tmux(Melding::Output { pane, data }) => {
                if self.valgt.as_ref().is_some_and(|p| p.id == pane) {
                    self.output(&data);
                }
                false
            }
            Hendelse::Tmux(Melding::Endret) => {
                self.list_paner();
                false
            }
            Hendelse::Tmux(Melding::Slutt(grunn)) => {
                self.ferdig = Some(if grunn.is_empty() { "tmux koblet fra".into() } else { grunn });
                false
            }
            Hendelse::Term(Event::Key(k)) if k.kind == KeyEventKind::Press => self.tast(k),
            Hendelse::Term(Event::Resize(..)) => true,
            Hendelse::Term(_) => false,
        }
    }

    fn output(&mut self, data: &str) {
        self.utdatert = true;
        let søk = std::mem::take(&mut self.output_hale) + data;
        if let Some(synk) = synkronisert_etter(&søk) {
            if synk && !self.synk {
                self.synk_siden = Instant::now();
            }
            self.synk = synk;
        }
        // Markøren er 11 tegn, så de siste 10 holder for å fange en som er
        // delt mellom to biter.
        let fra = søk.char_indices().rev().nth(9).map_or(0, |(i, _)| i);
        self.output_hale = søk[fra..].to_string();
    }

    fn svar(&mut self, venter: Venter, ok: bool, linjer: Vec<String>) -> bool {
        match venter {
            Venter::Paner => {
                self.lister = false;
                self.sist_listet = Some(Instant::now());
                self.paner = linjer
                    .iter()
                    .filter_map(|l| Pane::fra_linje(l))
                    .filter(|p| ok && !p.er_sidebar)
                    .collect();
                let valgt = self.valgt.as_ref().and_then(|v| self.paner.iter().find(|p| p.id == v.id));
                match valgt.cloned() {
                    // Oppdatert, så størrelsen regnes fra det tmux sier nå.
                    Some(pane) => self.valgt = Some(pane),
                    None => {
                        if let Some(første) = self.paner.first().cloned() {
                            self.velg(første);
                        } else {
                            self.valgt = None;
                        }
                    }
                }
                true
            }
            Venter::Bilde => {
                if ok {
                    self.bilde = sgr::tolk(&linjer);
                }
                false
            }
            Venter::Markør => {
                self.henter = false;
                let felt: Vec<&str> = linjer.first().map_or("", |l| l.as_str()).split('\t').collect();
                self.markør = match felt.as_slice() {
                    [x, y, "1"] => x.parse().ok().zip(y.parse().ok()),
                    _ => None,
                };
                true
            }
            Venter::Ingenting => false,
        }
    }

    fn list_paner(&mut self) {
        if !self.lister {
            self.lister = true;
            self.send(&format!("list-panes -a -F {}", sitat(FELT)), Venter::Paner);
        }
    }

    fn velg(&mut self, pane: Pane) {
        if self.valgt.as_ref().map(|p| &p.sesjon_id) != Some(&pane.sesjon_id) {
            self.send(&format!("switch-client -t {}", sitat(&pane.sesjon_id)), Venter::Ingenting);
        }
        // Vinduet må være sesjonens aktive for å få størrelsen fra denne
        // klienten. Det bytter også vindu for andre klienter i samme sesjon.
        self.send(&format!("select-window -t {}", sitat(&pane.vindu_id)), Venter::Ingenting);
        self.valgt = Some(pane);
        self.bilde.clear();
        self.markør = None;
        self.størrelse = None;
        self.utdatert = true;
        self.synk = false;
        self.output_hale.clear();
    }

    fn hent(&mut self) {
        let Some(id) = self.valgt.as_ref().map(|p| sitat(&p.id)) else { return };
        self.send(&format!("capture-pane -p -e -N -t {id}"), Venter::Bilde);
        let felt = sitat("#{cursor_x}\t#{cursor_y}\t#{cursor_flag}");
        self.send(&format!("display-message -p -t {id} {felt}"), Venter::Markør);
        self.utdatert = false;
        self.henter = true;
        self.sist_hentet = Instant::now();
    }

    /// Klienten får størrelsen som gjør at den valgte panen fyller visningen.
    /// Resten av vinduet, som sidebar-kolonnene fra tmux-oppsettet, legges til.
    fn synk_størrelse(&mut self) {
        let Some(pane) = &self.valgt else { return };
        let ønsket = (
            self.visning.width + pane.vindu_bredde.saturating_sub(pane.bredde),
            self.visning.height + pane.vindu_høyde.saturating_sub(pane.høyde),
        );
        if self.størrelse != Some(ønsket) && self.visning.width > 0 {
            self.send(&format!("refresh-client -C {}x{}", ønsket.0, ønsket.1), Venter::Ingenting);
            self.størrelse = Some(ønsket);
            self.utdatert = true;
        }
    }

    fn tast(&mut self, k: KeyEvent) -> bool {
        if taster::er_bytt_fokus(&k) {
            self.fokus = match self.fokus {
                Fokus::Liste => Fokus::Pane,
                Fokus::Pane => Fokus::Liste,
            };
            return true;
        }
        if self.fokus == Fokus::Pane {
            if let (Some(pane), Some(args)) = (&self.valgt, taster::send_keys(&k)) {
                let kommando = format!("send-keys -t {} {args}", sitat(&pane.id));
                self.send(&kommando, Venter::Ingenting);
            }
            return false;
        }
        match k.code {
            KeyCode::Char('q') => self.ferdig = Some(String::new()),
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                self.ferdig = Some(String::new())
            }
            KeyCode::Up | KeyCode::Char('k') => self.flytt(-1),
            KeyCode::Down | KeyCode::Char('j') => self.flytt(1),
            KeyCode::Enter => self.fokus = Fokus::Pane,
            _ => return false,
        }
        true
    }

    fn flytt(&mut self, steg: isize) {
        if self.paner.is_empty() {
            return;
        }
        let nå = self
            .valgt
            .as_ref()
            .and_then(|v| self.paner.iter().position(|p| p.id == v.id))
            .unwrap_or(0);
        let ny = (nå as isize + steg).clamp(0, self.paner.len() as isize - 1) as usize;
        if ny != nå {
            self.velg(self.paner[ny].clone());
        }
    }
}
