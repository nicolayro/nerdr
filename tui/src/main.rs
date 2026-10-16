mod grupper;
mod hjelp;
mod ny;
mod taster;
mod tegn;

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, io, thread};

use grupper::{Gruppe, Merke, Rad};
use nerdr_kjerne::kontroll::{Kontroll, Melding, dobbelt_sitat, sitat, synkronisert_etter};
use nerdr_kjerne::paner::{FELT, Pane};
use nerdr_kjerne::git;
use nerdr_kjerne::register::{self, Sesjon};
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
    /// Svaret er pane-id-en til den nye sesjonen, eller en feil når den finnes.
    NySesjon { navn: String, mappe: PathBuf, claude: &'static str },
    NyPane { mappe: PathBuf, claude: &'static str },
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
    list_igjen: bool,
    sist_listet: Option<Instant>,
    /// Sesjonsregisteret til Claude Code.
    register: PathBuf,
    /// Pane-id → sesjon, for panene der en Claude kjører.
    sesjoner: HashMap<String, Sesjon>,
    /// Pane-id → branchen mappen til sesjonen står på.
    brancher: HashMap<String, String>,
    /// Pane-id → da du sist var inne i panen. En Claude nerdr ikke har sett
    /// før, regnes som sett, så ingenting havner i «venter på deg» ved oppstart.
    sett: HashMap<String, u64>,
    /// Panen du er inne i, og gruppen den sto i da du gikk inn. Den blir stående
    /// der til du går ut, så kortet ikke flytter seg for hver melding.
    låst: Option<(String, Gruppe)>,

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
    /// Mappevelgeren, når den er oppe.
    velger: Option<ny::Velger>,
    hjelp: bool,
    /// Pane-id → mappe for Claude-er nerdr har startet, til de står i
    /// registeret. Oppstarten tar flere sekunder, og så lenge vises de likevel.
    nye: HashMap<String, PathBuf>,
    /// Panen nerdr skal gå inn i når den dukker opp i `list-panes`.
    gå_inn_i: Option<String>,
    ferdig: Option<String>,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let socket = match args.as_slice() {
        [] => None,
        [flagg, socket] if flagg == "-L" => Some(socket.as_str()),
        [flagg] if flagg == "-h" || flagg == "--help" => {
            print!("{}", hjelp::tekst());
            return;
        }
        [flagg] if flagg == "-V" || flagg == "--version" => {
            println!("nerdr {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        _ => {
            eprintln!("bruk: nerdr [-L socket], eller nerdr --help");
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
    })
    .map_err(|feil| match feil.kind() {
        io::ErrorKind::NotFound => io::Error::other("fant ikke tmux. Installer den, for eksempel med `brew install tmux`."),
        _ => feil,
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
            list_igjen: false,
            sist_listet: None,
            register: registerkatalog(),
            sesjoner: HashMap::new(),
            brancher: HashMap::new(),
            sett: HashMap::new(),
            låst: None,
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
            velger: None,
            hjelp: false,
            nye: HashMap::new(),
            gå_inn_i: None,
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
            Hendelse::Term(Event::Paste(tekst)) => match &mut self.velger {
                Some(velger) => {
                    velger.skriv(&tekst);
                    true
                }
                None => {
                    self.lim_inn(&tekst);
                    false
                }
            },
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
                self.sesjoner = register::les(&self.register);
                let nå = nå_ms();
                self.sett.retain(|id, _| self.sesjoner.contains_key(id));
                for id in self.sesjoner.keys() {
                    self.sett.entry(id.clone()).or_insert(nå);
                }
                let alle: Vec<Pane> = linjer.iter().filter_map(|l| Pane::fra_linje(l)).collect();
                if ok {
                    self.nye.retain(|id, _| !self.sesjoner.contains_key(id) && alle.iter().any(|p| &p.id == id));
                }
                self.brancher = self
                    .sesjoner
                    .iter()
                    .filter_map(|(id, s)| Some((id, s.mappe.as_deref()?)))
                    .chain(self.nye.iter().map(|(id, m)| (id, m.as_path())))
                    .filter_map(|(id, mappe)| Some((id.clone(), git::branch(mappe)?)))
                    .collect();
                self.paner = alle
                    .into_iter()
                    .filter(|p| ok && (self.sesjoner.contains_key(&p.id) || self.nye.contains_key(&p.id)))
                    .collect();
                let valgt = self.valgt.as_ref().and_then(|v| self.paner.iter().find(|p| p.id == v.id));
                match valgt.cloned() {
                    // Oppdatert, så størrelsen regnes fra det tmux sier nå.
                    Some(pane) => self.valgt = Some(pane),
                    None => {
                        if let Some(første) = self.i_rekkefølge().first().cloned() {
                            self.velg(første);
                        } else {
                            self.valgt = None;
                        }
                    }
                }
                let ny = self.gå_inn_i.as_ref().and_then(|id| self.paner.iter().find(|p| &p.id == id));
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
            Venter::NySesjon { navn, mappe, claude } if !ok => {
                let mål = sitat(&format!("={navn}:"));
                let kommando =
                    format!("new-window -t {mål} -c {} -P -F {}", sitat(&mappe.to_string_lossy()), sitat("#{pane_id}"));
                self.send(&kommando, Venter::NyPane { mappe, claude });
                false
            }
            Venter::NySesjon { mappe, claude, .. } | Venter::NyPane { mappe, claude } => {
                let Some(id) = linjer.into_iter().next().filter(|_| ok) else { return false };
                // Skrevet inn i skallet heller enn kjørt av tmux, siden `claude`
                // kan være en funksjon i .zshrc, som tmux ikke ser.
                self.send(&format!("send-keys -t {} -l {}", sitat(&id), sitat(claude)), Venter::Ingenting);
                self.send(&format!("send-keys -t {} Enter", sitat(&id)), Venter::Ingenting);
                self.nye.insert(id.clone(), mappe);
                self.gå_inn_i = Some(id);
                self.list_paner();
                false
            }
            Venter::Ingenting => false,
        }
    }

    /// `claude` er kommandolinja som skrives inn, som `claude --resume`.
    fn start_claude(&mut self, mappe: PathBuf, claude: &'static str) {
        let navn = ny::sesjonsnavn(&mappe);
        let kommando = format!(
            "new-session -d -s {} -c {} -P -F {}",
            sitat(&navn),
            sitat(&mappe.to_string_lossy()),
            sitat("#{pane_id}")
        );
        self.send(&kommando, Venter::NySesjon { navn, mappe, claude });
    }

    /// Er en liste alt på vei, kan den være bestilt før endringen, så det
    /// listes en gang til når den kommer.
    fn list_paner(&mut self) {
        if self.lister {
            self.list_igjen = true;
        } else {
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
    /// Resten av vinduet, som paner ved siden av den, legges til.
    ///
    /// Sendes bare når visningen endres eller en ny pane velges, aldri fordi
    /// panene i tmux har endret størrelse. Et tmux-oppsett med en
    /// `client-resized`-hook som endrer panene, gir ellers en løkke.
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
        if self.hjelp {
            self.hjelp = false;
            return true;
        }
        if let Some(velger) = &mut self.velger {
            let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
            match k.code {
                KeyCode::Esc => self.velger = None,
                KeyCode::Char('c') if ctrl => self.velger = None,
                KeyCode::Enter => {
                    if let Some(mappe) = velger.valgt_mappe() {
                        self.velger = None;
                        self.start_claude(mappe, "claude");
                    }
                }
                // Claude viser selv de tidligere samtalene i mappa å velge mellom.
                KeyCode::Char('r') if ctrl => {
                    if let Some(mappe) = velger.valgt_mappe() {
                        self.velger = None;
                        self.start_claude(mappe, "claude --resume");
                    }
                }
                KeyCode::Up => velger.flytt(-1),
                KeyCode::Char('p' | 'k') if ctrl => velger.flytt(-1),
                KeyCode::Down => velger.flytt(1),
                KeyCode::Char('n' | 'j') if ctrl => velger.flytt(1),
                KeyCode::Char('u') if ctrl => velger.søk.clear(),
                KeyCode::Backspace => velger.slett(),
                KeyCode::Char(c) if !ctrl => velger.skriv(c.encode_utf8(&mut [0; 4])),
                _ => return false,
            }
            return true;
        }
        if taster::er_bytt_fokus(&k) {
            self.bytt_fokus(match self.fokus {
                Fokus::Liste => Fokus::Pane,
                Fokus::Pane => Fokus::Liste,
            });
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
            KeyCode::Enter => self.bytt_fokus(Fokus::Pane),
            KeyCode::Char('n') => self.velger = Some(ny::Velger::ny()),
            KeyCode::Char('?') => self.hjelp = true,
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
        if self.velger.is_some() || self.hjelp {
            return false;
        }
        let v = self.visning;
        if m.column < v.x {
            let pane = (m.row + 1 < v.height).then(|| tegn::pane_på(self, m.row)).flatten();
            let Some(pane) = pane else { return false };
            if m.kind != MouseEventKind::Down(MouseButton::Left) {
                return false;
            }
            // Via lista, så panen du forlater, merkes som sett før du går inn.
            self.bytt_fokus(Fokus::Liste);
            if self.valgt.as_ref().map(|p| &p.id) != Some(&pane.id) {
                self.velg(pane);
            }
            self.bytt_fokus(Fokus::Pane);
            return true;
        }

        let byttet = matches!(m.kind, MouseEventKind::Down(_)) && self.fokus != Fokus::Pane;
        if byttet {
            self.bytt_fokus(Fokus::Pane);
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
        let paner = self.i_rekkefølge();
        if paner.is_empty() {
            return;
        }
        let nå = self.valgt.as_ref().and_then(|v| paner.iter().position(|p| p.id == v.id)).unwrap_or(0);
        let ny = (nå as isize + steg).clamp(0, paner.len() as isize - 1) as usize;
        if ny != nå {
            self.velg(paner[ny].clone());
        }
    }

    fn bytt_fokus(&mut self, fokus: Fokus) {
        if fokus == self.fokus {
            return;
        }
        // Gruppen må tas før panen merkes som sett, ellers havner en ferdig
        // Claude du går inn i, rett i SETT.
        self.låst = match fokus {
            Fokus::Pane => self.valgt.as_ref().and_then(|p| Some((p.id.clone(), self.merke(p)?.gruppe()))),
            Fokus::Liste => None,
        };
        if let Some(pane) = &self.valgt {
            self.sett.insert(pane.id.clone(), nå_ms());
        }
        self.fokus = fokus;
    }

    fn merke(&self, pane: &Pane) -> Option<Merke> {
        let Some(sesjon) = self.sesjoner.get(&pane.id) else {
            return self.nye.contains_key(&pane.id).then_some(Merke::Starter);
        };
        let inne = self.fokus == Fokus::Pane && self.valgt.as_ref().is_some_and(|v| v.id == pane.id);
        let sett = if inne { u64::MAX } else { self.sett.get(&pane.id).copied().unwrap_or(0) };
        Some(Merke::fra(sesjon, sett))
    }

    fn plass(&self, pane: &Pane) -> Option<(Gruppe, Merke)> {
        let merke = self.merke(pane)?;
        let gruppe = match &self.låst {
            Some((id, gruppe)) if *id == pane.id => *gruppe,
            _ => merke.gruppe(),
        };
        Some((gruppe, merke))
    }

    fn rader(&self) -> Vec<Rad<'_>> {
        grupper::rader(&self.paner, |p| self.plass(p))
    }

    fn i_rekkefølge(&self) -> Vec<Pane> {
        let rader = self.rader();
        rader.into_iter().filter_map(|r| if let Rad::Pane(p, _) = r { Some(p.clone()) } else { None }).collect()
    }
}

fn nå_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}
