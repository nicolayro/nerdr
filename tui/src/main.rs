mod bokser;
mod grupper;
mod hjelp;
mod kommandoer;
mod ny;
mod taster;
mod tegn;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, io, thread};

use bokser::Boks;
use grupper::{Gruppe, Merke, Rad};
use nerdr_kjerne::kontroll::{Kontroll, Melding, dobbelt_sitat, sitat, synkronisert_etter};
use nerdr_kjerne::paner::{FELT, Pane};
use nerdr_kjerne::{git, oppsett, start, terminal, tmux};
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
/// Linjer per hakk på hjulet, som piltaster i alternativ skjerm og som linjer
/// i historikken.
const RULL_STEG: u32 = 3;

enum Hendelse {
    Tmux(Melding),
    Term(Event),
}

/// Hva neste svar fra tmux gjelder. Svarene kommer i samme rekkefølge som
/// kommandoene, så køen holder rede på dem.
enum Venter {
    Paner,
    /// Innholdet i panen med id-en.
    Bilde(String),
    Markør,
    /// Svaret er pane-id-en til den nye sesjonen, eller en feil når den finnes.
    NySesjon { navn: String, mappe: PathBuf, claude: String },
    NyPane { mappe: PathBuf, claude: String },
    /// Svaret er pane-id-en til terminalen for Claude-panen `claude`.
    NyTerminal { claude: String },
    Prefiks,
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

    /// Claude-ene og terminalene som hører til dem, i lista.
    paner: Vec<Pane>,
    /// Alle panene på serveren.
    alle: Vec<Pane>,
    lister: bool,
    list_igjen: bool,
    sist_listet: Option<Instant>,
    /// Konfigurasjonskatalogen til Claude Code, med registeret i `sessions`
    /// og samtalene i `projects`.
    konfig: PathBuf,
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

    /// Panen som er valgt i lista. Vinduet den står i, vises til høyre.
    valgt: Option<Pane>,
    /// Panene i vinduet, der tmux har plassert dem.
    vindu: Vec<Pane>,
    /// Pane-id → innholdet.
    bilder: HashMap<String, Vec<Linje>>,
    /// Resten gjelder den aktive panen i vinduet, den tastene går til.
    markør: Option<(u16, u16)>,
    /// Panen har slått på mus med SGR-koding, slik Claude Code gjør i fullskjerm.
    pane_mus: bool,
    /// Panen viser den alternative skjermen, som `less` og `vim`, og den har
    /// ingen historikk.
    alternativ: bool,
    /// Linjer i historikken til panen, og hvor mange av dem visningen er rullet
    /// opp. 0 er bunnen, slik panen står.
    historie: u32,
    rull: u32,
    utdatert: bool,
    henter: bool,
    sist_hentet: Instant,
    /// Pane-id → siden når panen har vært midt i en synkronisert oppdatering.
    synk: HashMap<String, Instant>,
    /// Pane-id → slutten av forrige `%output`, i tilfelle en synkmarkør er
    /// delt over to.
    output_hale: HashMap<String, String>,
    /// Visningen og rutene boksene trengte, da klientstørrelsen sist ble sendt.
    størrelse_for: Option<(Rect, (u16, u16))>,

    fokus: Fokus,
    prefiks: taster::Prefiks,
    /// Prefikset er trykket, og neste tast er en kommando.
    etter_prefiks: bool,
    /// Panen som lukkes hvis du svarer `y`.
    lukke: Option<Pane>,
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
    let mut args: Vec<String> = env::args().skip(1).collect();
    let (Some(socket), Some(fortsett)) = (ta_verdi(&mut args, "-L"), ta_verdi(&mut args, "--fortsett")) else {
        bruk();
    };
    let json = ta_flagg(&mut args, "--json");
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let (socket, fortsett) = (socket.as_deref(), fortsett.as_deref());

    let res = match args.as_slice() {
        [] if !json && fortsett.is_none() => kjør(socket),
        ["liste"] if fortsett.is_none() => kommandoer::liste(socket, json),
        ["samtaler"] if fortsett.is_none() => kommandoer::samtaler(json),
        ["ny"] if fortsett.is_some() => kommandoer::ny(socket, None, fortsett, json),
        ["ny", mappe] => kommandoer::ny(socket, Some(mappe), fortsett, json),
        ["-h" | "--help"] => {
            print!("{}", hjelp::tekst());
            Ok(())
        }
        ["-v" | "-V" | "--version"] => {
            println!("nerdr {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => bruk(),
    };
    if let Err(feil) = res {
        eprintln!("nerdr: {}", uten_tmux(feil));
        std::process::exit(1);
    }
}

fn bruk() -> ! {
    eprintln!("bruk: nerdr [liste | samtaler | ny <mappe>] [--json] [-L socket], eller nerdr --help");
    std::process::exit(2);
}

/// Fjerner `navn verdi` fra `args`. Ytterst `None` når verdien mangler.
fn ta_verdi(args: &mut Vec<String>, navn: &str) -> Option<Option<String>> {
    let Some(i) = args.iter().position(|a| a == navn) else { return Some(None) };
    if i + 1 >= args.len() {
        return None;
    }
    let verdi = args.remove(i + 1);
    args.remove(i);
    Some(Some(verdi))
}

fn ta_flagg(args: &mut Vec<String>, navn: &str) -> bool {
    let før = args.len();
    args.retain(|a| a != navn);
    args.len() < før
}

fn uten_tmux(feil: io::Error) -> io::Error {
    match feil.kind() {
        io::ErrorKind::NotFound => io::Error::other("fant ikke tmux. Installer den, for eksempel med `brew install tmux`."),
        _ => feil,
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
pub fn konfigkatalog() -> PathBuf {
    env::var("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env::var("HOME").unwrap_or_default()).join(".claude"))
}

impl App {
    fn ny(kontroll: Kontroll) -> App {
        App {
            kontroll,
            kø: VecDeque::new(),
            paner: Vec::new(),
            alle: Vec::new(),
            lister: false,
            list_igjen: false,
            sist_listet: None,
            konfig: konfigkatalog(),
            sesjoner: HashMap::new(),
            brancher: HashMap::new(),
            sett: HashMap::new(),
            låst: None,
            valgt: None,
            vindu: Vec::new(),
            bilder: HashMap::new(),
            markør: None,
            pane_mus: false,
            alternativ: false,
            historie: 0,
            rull: 0,
            utdatert: false,
            henter: false,
            sist_hentet: Instant::now(),
            synk: HashMap::new(),
            output_hale: HashMap::new(),
            størrelse_for: None,
            fokus: Fokus::Liste,
            prefiks: taster::Prefiks::default(),
            etter_prefiks: false,
            lukke: None,
            visning: Rect::default(),
            velger: None,
            hjelp: false,
            nye: HashMap::new(),
            gå_inn_i: None,
            ferdig: None,
        }
    }

    fn løkke(&mut self, terminal: &mut DefaultTerminal, rx: &Receiver<Hendelse>) -> io::Result<()> {
        self.send("show-options -gv prefix", Venter::Prefiks);
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
        let synk = self.synk.values().min().map_or(Duration::ZERO, |siden| SYNK_MAKS.saturating_sub(siden.elapsed()));
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
                if self.vindu.iter().any(|p| p.id == pane) {
                    self.output(pane, &data);
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

    fn output(&mut self, pane: String, data: &str) {
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

    fn svar(&mut self, venter: Venter, ok: bool, linjer: Vec<String>) -> bool {
        match venter {
            Venter::Paner => {
                self.lister = false;
                self.sist_listet = Some(Instant::now());
                self.sesjoner = register::les(&self.konfig.join("sessions"));
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
                let claude: HashSet<&str> =
                    alle.iter().map(|p| p.id.as_str()).filter(|id| self.er_claude(id)).collect();
                let synlig = |p: &Pane| claude.contains(p.id.as_str()) || p.claude.as_deref().is_some_and(|c| claude.contains(c));
                self.paner = alle.iter().filter(|p| ok && synlig(p)).cloned().collect();
                self.alle = alle;
                let valgt = self.valgt.as_ref().and_then(|v| self.paner.iter().find(|p| p.id == v.id));
                match valgt.cloned() {
                    // Oppdatert, så størrelsen regnes fra det tmux sier nå.
                    Some(pane) => {
                        self.valgt = Some(pane);
                        self.oppdater_vindu(None);
                    }
                    None => {
                        // Tastene du er midt i, skal ikke gå til en annen pane.
                        self.bytt_fokus(Fokus::Liste);
                        let eier = self.valgt.as_ref().and_then(|v| v.claude.as_ref());
                        let eier = eier.and_then(|c| self.paner.iter().find(|p| &p.id == c));
                        if let Some(pane) = eier.or(self.i_rekkefølge().first()).cloned() {
                            self.velg(pane);
                        } else {
                            self.valgt = None;
                            self.vindu.clear();
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
            Venter::Bilde(id) => {
                if ok && self.vindu.iter().any(|p| p.id == id) {
                    self.bilder.insert(id, sgr::tolk(&linjer));
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
                self.alternativ = felt.get(4) == Some(&"1");
                let historie: u32 = felt.get(5).and_then(|h| h.parse().ok()).unwrap_or(0);
                // Det du leser, blir stående når ny tekst skyver historikken opp.
                let rull = match self.rull {
                    _ if self.alternativ => 0,
                    0 => 0,
                    rull => rull.saturating_add(historie.saturating_sub(self.historie)).min(historie),
                };
                self.historie = historie;
                if rull != self.rull {
                    self.rull = rull;
                    self.utdatert = true;
                }
                true
            }
            Venter::NySesjon { navn, mappe, claude } if !ok => {
                self.send(&tmux::linje(&start::nytt_vindu(&navn, &mappe)), Venter::NyPane { mappe, claude });
                false
            }
            Venter::NySesjon { mappe, claude, .. } | Venter::NyPane { mappe, claude } => {
                let Some(id) = linjer.into_iter().next().filter(|_| ok) else { return false };
                for kommando in start::skriv_inn(&id, &claude) {
                    self.send(&tmux::linje(&kommando), Venter::Ingenting);
                }
                self.nye.insert(id.clone(), mappe);
                self.gå_inn_i = Some(id);
                self.list_paner();
                false
            }
            Venter::NyTerminal { claude } => {
                let Some(id) = linjer.into_iter().next().filter(|_| ok) else { return false };
                self.send(&tmux::linje(&terminal::knytt(&id, &claude)), Venter::Ingenting);
                self.gå_inn_i = Some(id);
                self.list_paner();
                false
            }
            Venter::Prefiks => {
                if let Some(prefiks) = linjer.first().filter(|_| ok).and_then(|l| taster::Prefiks::fra_tmux(l)) {
                    self.prefiks = prefiks;
                }
                false
            }
            Venter::Ingenting => false,
        }
    }

    /// Tasten etter prefikset. Tastene er de samme som i tmux, og virker
    /// både fra lista og fra panen.
    fn prefikstast(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let alt = k.modifiers.contains(KeyModifiers::ALT);
        match k.code {
            KeyCode::Char('w' | 's') if !ctrl => return self.bytt_fokus(Fokus::Liste),
            KeyCode::Char('n') if !ctrl => return self.bla(1),
            KeyCode::Char('p') if !ctrl => return self.bla(-1),
            KeyCode::Char('c') if !ctrl => return self.ny_terminal(),
            KeyCode::Char('d') if !ctrl => return self.ferdig = Some(String::new()),
            KeyCode::Char('?') => return self.hjelp = true,
            _ => {}
        }
        let (Some(aktiv), Some(vindu)) = (self.aktiv().cloned(), self.valgt.as_ref().map(|p| sitat(&p.vindu_id))) else {
            return;
        };
        let id = sitat(&aktiv.id);
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
            (_, Some(retning)) if ctrl || alt => format!("resize-pane {retning} -t {id} {}", if alt { 5 } else { 1 }),
            (_, Some(retning)) => format!("select-pane {retning} -t {id}"),
            (KeyCode::Char('%'), _) => format!("split-window -h -t {id} -c '#{{pane_current_path}}'"),
            (KeyCode::Char('"'), _) => format!("split-window -v -t {id} -c '#{{pane_current_path}}'"),
            (KeyCode::Char('o'), _) if ctrl => format!("rotate-window -t {vindu}"),
            (KeyCode::Char('o'), _) => {
                let i = self.vindu.iter().position(|p| p.aktiv).unwrap_or(0);
                if let Some(neste) = self.vindu.get((i + 1) % self.vindu.len().max(1)).map(|p| p.id.clone()) {
                    self.aktiver(&neste);
                }
                return;
            }
            (KeyCode::Char(';'), _) => format!("last-pane -t {vindu}"),
            (KeyCode::Char('z'), _) => format!("resize-pane -Z -t {id}"),
            (KeyCode::Char('{'), _) => format!("swap-pane -U -t {id}"),
            (KeyCode::Char('}'), _) => format!("swap-pane -D -t {id}"),
            (KeyCode::Char(' '), _) => format!("next-layout -t {vindu}"),
            (KeyCode::Char('x'), _) => return self.lukke = Some(aktiv),
            (KeyCode::PageUp, _) => {
                let side = self.bokser().iter().find(|b| b.pane == aktiv.id).map_or(1, |b| b.innhold.height.max(1));
                return self.rull_til(self.rull.saturating_add(side as u32));
            }
            _ => return,
        };
        self.send(&kommando, Venter::Ingenting);
    }

    /// Neste eller forrige i lista. Fra panen går du inn i den, som `n` og
    /// `p` bytter vindu i tmux.
    fn bla(&mut self, steg: isize) {
        let inne = self.fokus == Fokus::Pane;
        // Via lista, så panen du forlater, merkes som sett.
        self.bytt_fokus(Fokus::Liste);
        self.flytt(steg);
        if inne {
            self.bytt_fokus(Fokus::Pane);
        }
    }

    /// Går inn i terminalen til den valgte Claude-en, eller den som hører til
    /// samme Claude som den valgte terminalen. Finnes den ikke, lages den.
    fn åpne_terminal(&mut self) {
        let Some(valgt) = self.valgt.clone() else { return };
        let claude = self.eier(&valgt).unwrap_or(&valgt.id).to_string();
        if let Some(t) = self.paner.iter().find(|p| self.eier(p) == Some(&claude)).cloned() {
            if t.id != valgt.id {
                self.velg(t);
            }
            self.bytt_fokus(Fokus::Pane);
            return;
        }
        self.lag_terminal(claude);
    }

    /// En terminal til, til Claude-en som er valgt eller som den valgte
    /// terminalen hører til.
    fn ny_terminal(&mut self) {
        let Some(valgt) = self.valgt.clone() else { return };
        // Den nye går du inn i når den dukker opp, og det skjer fra lista.
        self.bytt_fokus(Fokus::Liste);
        self.lag_terminal(self.eier(&valgt).unwrap_or(&valgt.id).to_string());
    }

    fn lag_terminal(&mut self, claude: String) {
        let Some(vindu) = self.paner.iter().find(|p| p.id == claude).map(|p| p.vindu_id.clone()) else { return };
        let mappe = self.sesjoner.get(&claude).and_then(|s| s.mappe.clone()).or_else(|| self.nye.get(&claude).cloned());
        self.send(&tmux::linje(&terminal::ny(&vindu, mappe.as_deref())), Venter::NyTerminal { claude });
    }

    fn er_claude(&self, id: &str) -> bool {
        self.sesjoner.contains_key(id) || self.nye.contains_key(id)
    }

    /// Claude-panen `pane` er terminal for, eller `None` når den ikke er det.
    fn eier<'a>(&self, pane: &'a Pane) -> Option<&'a str> {
        pane.claude.as_deref().filter(|_| !self.er_claude(&pane.id))
    }

    /// Samme kommandoer som `start::start`, men gjennom control mode, så svaret
    /// kommer som en melding.
    fn start_claude(&mut self, mappe: PathBuf, samtale: Option<&str>) {
        let navn = start::sesjonsnavn(&mappe);
        let claude = start::claude(samtale);
        self.send(&tmux::linje(&start::ny_sesjon(&navn, &mappe)), Venter::NySesjon { navn, mappe, claude });
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
        self.send(&format!("select-pane -t {}", sitat(&pane.id)), Venter::Ingenting);
        let id = pane.id.clone();
        self.valgt = Some(pane);
        self.bilder.clear();
        self.synk.clear();
        self.output_hale.clear();
        self.størrelse_for = None;
        self.oppdater_vindu(Some(&id));
        self.nullstill_aktiv();
    }

    /// Henter panene i vinduet til den valgte fra `alle`. `aktiv` overstyrer
    /// den aktive panen, for et svar fra tmux som kan være eldre enn
    /// `select-pane`.
    fn oppdater_vindu(&mut self, aktiv: Option<&str>) {
        let Some(valgt) = &self.valgt else { return };
        let før = self.aktiv().map(|p| p.id.clone());
        self.vindu = self.alle.iter().filter(|p| p.vindu_id == valgt.vindu_id).cloned().collect();
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
    fn bokser(&self) -> Vec<Boks> {
        let v = self.visning;
        if let Some(oppsett) = self.oppsett() {
            return bokser::plasser(&oppsett, v);
        }
        let boks = |p: &Pane| Boks {
            pane: p.id.clone(),
            ramme: None,
            innhold: Rect::new(v.x + p.venstre, v.y + p.topp, p.bredde, p.høyde).intersection(v),
        };
        self.vindu.iter().map(boks).collect()
    }

    fn oppsett(&self) -> Option<oppsett::Celle> {
        oppsett::tolk(&self.vindu.first()?.oppsett)
    }

    /// Den aktive panen i vinduet, eller den valgte før vinduet er listet.
    fn aktiv(&self) -> Option<&Pane> {
        self.vindu.iter().find(|p| p.aktiv).or(self.valgt.as_ref())
    }

    fn aktiver(&mut self, id: &str) {
        self.send(&format!("select-pane -t {}", sitat(id)), Venter::Ingenting);
        for p in &mut self.vindu {
            p.aktiv = p.id == id;
        }
        self.nullstill_aktiv();
    }

    fn nullstill_aktiv(&mut self) {
        self.markør = None;
        self.pane_mus = false;
        self.alternativ = false;
        self.historie = 0;
        self.rull = 0;
        self.utdatert = true;
    }

    fn hent(&mut self) {
        let Some(aktiv) = self.aktiv().map(|p| p.id.clone()) else { return };
        let mut ider: Vec<String> = self.bokser().into_iter().map(|b| b.pane).collect();
        if ider.is_empty() {
            ider.push(aktiv.clone());
        }
        for id in ider {
            // Fra -S og ned til bunnen er flere linjer enn panen, og tegningen
            // tar de øverste.
            let fra = if id == aktiv && self.rull > 0 { format!(" -S -{}", self.rull) } else { String::new() };
            self.send(&format!("capture-pane -p -e -N -t {}{fra}", sitat(&id)), Venter::Bilde(id));
        }
        let id = sitat(&aktiv);
        let felt = sitat(
            "#{cursor_x}\t#{cursor_y}\t#{cursor_flag}\t#{&&:#{mouse_any_flag},#{mouse_sgr_flag}}\t\
             #{alternate_on}\t#{history_size}",
        );
        self.send(&format!("display-message -p -t {id} {felt}"), Venter::Markør);
        self.utdatert = false;
        self.henter = true;
        self.sist_hentet = Instant::now();
    }

    /// Klienten får størrelsen til visningen, minus rutene rammene rundt
    /// boksene trenger, så vinduet fyller visningen.
    ///
    /// Sendes bare når visningen endres, en ny pane velges eller vinduet deles
    /// på en annen måte, aldri fordi panene i tmux har endret størrelse. Et
    /// tmux-oppsett med en `client-resized`-hook som endrer panene, gir ellers
    /// en løkke.
    fn synk_størrelse(&mut self) {
        let v = self.visning;
        let ekstra = self.oppsett().map_or((0, 0), |o| bokser::ekstra(&o));
        if self.valgt.is_none() || self.størrelse_for == Some((v, ekstra)) || v.width <= ekstra.0 || v.height <= ekstra.1 {
            return;
        }
        let (bredde, høyde) = (v.width - ekstra.0, v.height - ekstra.1);
        self.send(&format!("refresh-client -C {bredde}x{høyde}"), Venter::Ingenting);
        self.størrelse_for = Some((v, ekstra));
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
                    if let Some(valgt) = velger.valgt() {
                        let (mappe, samtale) = (valgt.mappe.clone(), valgt.samtale.clone());
                        self.velger = None;
                        self.start_claude(mappe, samtale.as_deref());
                    }
                }
                KeyCode::Char('r') if ctrl => velger.bytt(),
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
        if let Some(pane) = self.lukke.take() {
            if k.code == KeyCode::Char('y') {
                self.send(&format!("kill-pane -t {}", sitat(&pane.id)), Venter::Ingenting);
            }
            return true;
        }
        if std::mem::take(&mut self.etter_prefiks) {
            self.prefikstast(k);
            return true;
        }
        if self.prefiks.er(&k) {
            self.etter_prefiks = true;
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
            if let (Some(pane), Some(args)) = (self.aktiv(), taster::send_keys(&k)) {
                let kommando = format!("send-keys -t {} {args}", sitat(&pane.id));
                self.send(&kommando, Venter::Ingenting);
                self.rull_til(0);
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
            KeyCode::Char('n') => self.åpne_velger(false),
            KeyCode::Char('t') => self.åpne_terminal(),
            KeyCode::Char('r') if k.modifiers.contains(KeyModifiers::CONTROL) => self.åpne_velger(true),
            KeyCode::Char('?') => self.hjelp = true,
            _ => return false,
        }
        true
    }

    fn åpne_velger(&mut self, fortsett: bool) {
        let kjører: HashSet<String> = self.sesjoner.values().filter_map(|s| s.samtale.clone()).collect();
        self.velger = Some(ny::Velger::ny(fortsett, &self.konfig.join("projects"), &kjører));
    }

    /// `paste-buffer -p` legger på markørene for bracketed paste bare når panen
    /// har bedt om dem, og uten dem sender Claude inn ved første linjeskift.
    fn lim_inn(&mut self, tekst: &str) {
        let Some(pane) = self.aktiv().filter(|_| self.fokus == Fokus::Pane) else { return };
        let id = sitat(&pane.id);
        // paste-buffer gjør LF om til CR, så CRLF ville blitt to linjeskift.
        let tekst = tekst.replace("\r\n", "\n");
        self.send(&format!("set-buffer -b nerdr {}", dobbelt_sitat(&tekst)), Venter::Ingenting);
        self.send(&format!("paste-buffer -p -d -b nerdr -t {id}"), Venter::Ingenting);
        self.rull_til(0);
    }

    fn rull_til(&mut self, rull: u32) {
        let rull = rull.min(self.historie);
        if rull != self.rull {
            self.rull = rull;
            self.utdatert = true;
        }
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
        let (id, x, y) = (sitat(&pane.id), m.column - boks.innhold.x, m.row - boks.innhold.y);
        let opp = match m.kind {
            MouseEventKind::ScrollUp => Some(true),
            MouseEventKind::ScrollDown => Some(false),
            _ => None,
        };
        match opp {
            _ if self.pane_mus => {
                if let Some(args) = taster::mus(&m, x, y) {
                    self.send(&format!("send-keys -t {id} {args}"), Venter::Ingenting);
                }
            }
            Some(opp) if self.alternativ => {
                let tast = if opp { "Up" } else { "Down" };
                self.send(&format!("send-keys -t {id} -N {RULL_STEG} {tast}"), Venter::Ingenting);
            }
            Some(true) => self.rull_til(self.rull.saturating_add(RULL_STEG)),
            Some(false) => self.rull_til(self.rull.saturating_sub(RULL_STEG)),
            None => {}
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
        // Fra lista går du inn i panen du valgte, også når en annen er aktiv
        // i vinduet.
        let valgt = self.valgt.as_ref().map(|p| p.id.clone());
        if fokus == Fokus::Pane
            && let Some(id) = valgt.filter(|id| self.aktiv().is_some_and(|a| &a.id != id))
        {
            self.aktiver(&id);
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
        rader
            .into_iter()
            .filter_map(|r| match r {
                Rad::Pane(p, _) | Rad::Terminal(p) => Some(p.clone()),
                _ => None,
            })
            .collect()
    }
}

fn nå_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}
