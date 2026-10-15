mod taster;
mod tegn;

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};
use std::{env, io, thread};

use nerdr_kjerne::kontroll::{Kontroll, Melding, dobbelt_sitat, sitat, synkronisert_etter};
use nerdr_kjerne::paner::{FELT, Pane};
use nerdr_kjerne::register::{self, Tilstand};
use nerdr_kjerne::sgr::{self, Linje};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use ratatui::layout::Rect;

const LISTE_INTERVALL: Duration = Duration::from_secs(2);
/// Øvre grense for hvor ofte panen hentes på nytt, omtrent 60 bilder i sekundet.
const HENTE_INTERVALL: Duration = Duration::from_millis(16);
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
    /// Sesjonsregisteret til Claude Code.
    register: PathBuf,
    /// Pane-id → tilstand, for panene der en Claude kjører.
    tilstander: HashMap<String, Tilstand>,

    /// Panen som er valgt i lista og vises til høyre.
    valgt: Option<Pane>,
    bilde: Vec<Linje>,
    markør: Option<(u16, u16)>,
    /// Panen har slått på mus med SGR-koding, slik Claude Code gjør i fullskjerm.
    pane_mus: bool,
    utdatert: bool,
    henter: bool,
    sist_hentet: Instant,
    /// Den viste panen er midt i en synkronisert oppdatering.
    synk: bool,
    synk_siden: Instant,
    /// Slutten av forrige `%output`, i tilfelle en synkmarkør er delt over to.
    output_hale: String,
    /// Visningen klientstørrelsen sist ble sendt for.
    størrelse_for: Option<Rect>,

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
    execute!(io::stdout(), EnableMouseCapture, EnableBracketedPaste)?;
    let mut app = App::ny(kontroll);
    let res = app.løkke(&mut terminal, &rx);
    let _ = execute!(io::stdout(), DisableMouseCapture, DisableBracketedPaste);
    ratatui::restore();

    res?;
    match app.ferdig {
        Some(grunn) if !grunn.is_empty() => Err(io::Error::other(grunn)),
        _ => Ok(()),
    }
}

/// Samme katalog som Claude Code bruker: `$CLAUDE_CONFIG_DIR`, ellers `~/.claude`.
fn registerkatalog() -> PathBuf {
    let konfig = env::var("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env::var("HOME").unwrap_or_default()).join(".claude"));
    konfig.join("sessions")
}

impl App {
    fn ny(kontroll: Kontroll) -> App {
        App {
            kontroll,
            kø: VecDeque::new(),
            paner: Vec::new(),
            lister: false,
            sist_listet: None,
            register: registerkatalog(),
            tilstander: HashMap::new(),
            valgt: None,
            bilde: Vec::new(),
            markør: None,
            pane_mus: false,
            utdatert: false,
            henter: false,
            sist_hentet: Instant::now(),
            synk: false,
            synk_siden: Instant::now(),
            output_hale: String::new(),
            størrelse_for: None,
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
                // Når teksten i panen ruller, skrives nesten hver celle på nytt.
                // Uten synkronisert oppdatering kan terminalen vise den halvveis.
                execute!(terminal.backend_mut(), BeginSynchronizedUpdate)?;
                terminal.draw(|f| tegn::alt(f, self))?;
                execute!(terminal.backend_mut(), EndSynchronizedUpdate)?;
                self.synk_størrelse();
                tegne = false;
            }

            let vent = self.tid_til_henting().unwrap_or(HENTE_INTERVALL);
            let første = match rx.recv_timeout(vent) {
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
            if self.tid_til_henting() == Some(Duration::ZERO) {
                self.hent();
            }
        }
        Ok(())
    }

    /// Hvor lenge det er til panen kan hentes på nytt, eller `None` når det
    /// ikke er noe nytt å hente eller en henting allerede pågår.
    fn tid_til_henting(&self) -> Option<Duration> {
        if !self.utdatert || self.henter {
            return None;
        }
        let intervall = HENTE_INTERVALL.saturating_sub(self.sist_hentet.elapsed());
        let synk = if self.synk { SYNK_MAKS.saturating_sub(self.synk_siden.elapsed()) } else { Duration::ZERO };
        Some(intervall.max(synk))
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
            Hendelse::Term(Event::Mouse(m)) => self.mus(m),
            Hendelse::Term(Event::Paste(tekst)) => {
                self.lim_inn(&tekst);
                false
            }
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
                self.tilstander = register::les(&self.register);
                self.paner = linjer
                    .iter()
                    .filter_map(|l| Pane::fra_linje(l))
                    .filter(|p| ok && self.tilstander.contains_key(&p.id))
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
                    [x, y, "1", ..] => x.parse().ok().zip(y.parse().ok()),
                    _ => None,
                };
                self.pane_mus = felt.get(3) == Some(&"1");
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
        self.pane_mus = false;
        self.størrelse_for = None;
        self.utdatert = true;
        self.synk = false;
        self.output_hale.clear();
    }

    fn hent(&mut self) {
        let Some(id) = self.valgt.as_ref().map(|p| sitat(&p.id)) else { return };
        self.send(&format!("capture-pane -p -e -N -t {id}"), Venter::Bilde);
        let felt = sitat("#{cursor_x}\t#{cursor_y}\t#{cursor_flag}\t#{&&:#{mouse_any_flag},#{mouse_sgr_flag}}");
        self.send(&format!("display-message -p -t {id} {felt}"), Venter::Markør);
        self.utdatert = false;
        self.henter = true;
        self.sist_hentet = Instant::now();
    }

    /// Klienten får størrelsen som gjør at den valgte panen fyller visningen.
    /// Resten av vinduet, som sidebar-kolonnene fra tmux-oppsettet, legges til.
    ///
    /// Sendes bare når visningen endres eller en ny pane velges, aldri fordi
    /// panene i tmux har endret størrelse. `client-resized`-hooken i
    /// sidebar-repoet endrer panene som svar, og da ble det en løkke.
    fn synk_størrelse(&mut self) {
        let Some(pane) = &self.valgt else { return };
        if self.størrelse_for == Some(self.visning) || self.visning.width == 0 {
            return;
        }
        let bredde = self.visning.width + pane.vindu_bredde.saturating_sub(pane.bredde);
        let høyde = self.visning.height + pane.vindu_høyde.saturating_sub(pane.høyde);
        self.send(&format!("refresh-client -C {bredde}x{høyde}"), Venter::Ingenting);
        self.størrelse_for = Some(self.visning);
        self.utdatert = true;
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

    /// `paste-buffer -p` legger på markørene for bracketed paste bare når panen
    /// har bedt om dem, og uten dem sender Claude inn ved første linjeskift.
    fn lim_inn(&mut self, tekst: &str) {
        let Some(pane) = self.valgt.as_ref().filter(|_| self.fokus == Fokus::Pane) else { return };
        let id = sitat(&pane.id);
        // paste-buffer gjør LF om til CR, så CRLF ville blitt to linjeskift.
        let tekst = tekst.replace("\r\n", "\n");
        self.send(&format!("set-buffer -b nerdr {}", dobbelt_sitat(&tekst)), Venter::Ingenting);
        self.send(&format!("paste-buffer -p -d -b nerdr -t {id}"), Venter::Ingenting);
    }

    fn mus(&mut self, m: MouseEvent) -> bool {
        let v = self.visning;
        if m.column < v.x {
            let rad = (m.row + 1 < v.height).then(|| self.paner.get(m.row as usize)).flatten();
            let Some(pane) = rad.cloned() else { return false };
            if m.kind != MouseEventKind::Down(MouseButton::Left) {
                return false;
            }
            if self.valgt.as_ref().map(|p| &p.id) != Some(&pane.id) {
                self.velg(pane);
            }
            self.fokus = Fokus::Liste;
            return true;
        }

        let byttet = matches!(m.kind, MouseEventKind::Down(_)) && self.fokus != Fokus::Pane;
        if byttet {
            self.fokus = Fokus::Pane;
        }
        let (x, y) = (m.column - v.x, m.row - v.y);
        if self.pane_mus
            && x < v.width
            && y < v.height
            && let (Some(pane), Some(args)) = (&self.valgt, taster::mus(&m, x, y))
        {
            let kommando = format!("send-keys -t {} {args}", sitat(&pane.id));
            self.send(&kommando, Venter::Ingenting);
        }
        byttet
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
