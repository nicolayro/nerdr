mod bindinger;
mod bokser;
mod forklar;
mod grupper;
mod handlinger;
mod hjelp;
mod kommandoer;
mod markering;
mod ny;
mod palett;
mod taster;
mod tegn;
mod tekst;
mod utseende;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, io, thread};

use bokser::Boks;
use grupper::{Del, Plass, Rad};
use handlinger::{Handling, Handlinger};
use nerdr_kjerne::kontroll::{self, Kontroll, Melding, dobbelt_sitat, sitat, synkronisert_etter};
use nerdr_kjerne::paner::{self, FELT, Pane};
use nerdr_kjerne::{git, konfigkatalog, oppsett, start, tmux};
use nerdr_kjerne::merke::Merke;
use nerdr_kjerne::register::{self, Claude};
use nerdr_kjerne::sgr::{self, Linje};
use markering::{Markering, Punkt};
use utseende::{Orden, Utseende};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton,
    MouseEvent, MouseEventKind,
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
/// En Claude nerdr har startet, som ikke står i registeret etter så lang tid,
/// kom trolig aldri i gang, som når `claude` ikke finnes i skallet. Den som
/// starter tregt, er i registeret lenge før.
const STARTER_MAKS: Duration = Duration::from_secs(5 * 60);
/// Linjer per hakk på hjulet, som piltaster i alternativ skjerm og som linjer
/// i historikken.
const RULL_STEG: u32 = 3;
const KOPIERT_VISES: Duration = Duration::from_millis(1500);
/// Terminalen sender to klikk og sier ikke fra om dobbeltklikk.
const DOBBELTKLIKK: Duration = Duration::from_millis(500);
/// Global option i tmux med bredden på lista, og `stripe` etter når den er en
/// stripe, så den er den samme neste gang nerdr starter.
const LISTE_OPTION: &str = "@nerdr-liste";

enum Hendelse {
    Tmux(Melding),
    Term(Event),
    /// Prosjektene til mappevelgeren, lest i en egen tråd.
    Prosjekter(Vec<PathBuf>),
}

/// Hva neste svar fra tmux gjelder. Svarene kommer i samme rekkefølge som
/// kommandoene, så køen holder rede på dem.
enum Svar {
    Paner,
    /// Innholdet i panen med id-en.
    Bilde(String),
    Markør,
    /// Svaret er id-en til vinduet.
    NyttVindu,
    /// Svaret er pane-id-en til den nye sesjonen, eller en feil når den finnes.
    NySesjon { navn: String, mappe: PathBuf, claude_kommando: String },
    NyPane { mappe: PathBuf, claude_kommando: String },
    Prefiks,
    /// Prefikstabellen fra `list-keys`.
    Bindinger,
    Gjentatid,
    /// Bredden på lista fra `LISTE_OPTION`.
    Liste,
    Utseende,
    Ingenting,
}

/// Det nerdr spør om i en popup, og kommandoen som lukker det hvis du svarer
/// `y`.
struct Lukke {
    spørsmål: &'static str,
    hva: String,
    kommando: String,
}

/// Om tastene går til lista eller til den aktive panen i vinduet som vises.
#[derive(Clone, Copy, PartialEq)]
enum Fokus {
    Liste,
    Pane,
}

/// Det et klikk der musen står, ville truffet.
#[derive(PartialEq)]
enum Mål {
    /// Pane-id-en til Claude-en i lista.
    Kort(String),
    /// `×` på kortet, med pane-id-en.
    Lukk(String),
    /// «lukk» eller «avbryt» i popupen som spør.
    Svar(bool),
    /// Et treff i handlingene eller velgeren.
    Rad(usize),
    /// En verdi i konfigurasjonen, som aksen og indeksen.
    Verdi(usize, usize),
    /// Id-en til vinduet.
    Fane(String),
    /// `×` på fanen, med id-en til vinduet.
    LukkFane(String),
    /// En pane i vinduet som tastene ikke går til.
    Pane(String),
    Knapp(Knapp),
    /// Knappen øverst i lista som bytter sortering.
    Sortering,
    /// `+` etter fanene.
    NyFane,
    /// Streken mellom lista og panen, som kan dras.
    Skille,
}

/// Knappene nederst i lista.
#[derive(Clone, Copy, PartialEq)]
enum Knapp {
    Ny,
    Handlinger,
    Hjelp,
    Avslutt,
}

struct App {
    kontroll: Kontroll,
    /// tmux kobler fra klienten når sesjonen den står i, forsvinner, som når
    /// det siste vinduet i den lukkes. Da kobler nerdr til igjen.
    koble_igjen: Box<dyn FnMut() -> io::Result<Kontroll>>,
    koblet_igjen: Option<Instant>,
    /// For det som leses i egne tråder.
    tx: Sender<Hendelse>,
    kø: VecDeque<Svar>,

    /// Panene der en Claude kjører eller starter, i lista.
    claude_paner: Vec<Pane>,
    /// Alle panene på serveren.
    alle: Vec<Pane>,
    lister: bool,
    list_igjen: bool,
    sist_listet: Option<Instant>,
    /// Konfigurasjonskatalogen til Claude Code, med registeret i `sessions`
    /// og samtalene i `projects`.
    konfig: PathBuf,
    /// Pane-id → Claude-en som kjører i panen.
    claude: HashMap<String, Claude>,
    /// Lest bare når lista er tom, for det er da du lurer på hvor de er.
    utenfor_tmux: usize,
    /// Pane-id → repoet og branchen mappa til Claude-en står i.
    repoer: HashMap<String, git::Repo>,
    /// Pane-id → da du sist var inne i panen, også skrevet til panen i tmux.
    /// En Claude nerdr ikke har sett før, regnes som sett, så ingenting havner
    /// i «venter på deg» første gang nerdr starter.
    sett: HashMap<String, u64>,
    /// Panen du er inne i, og plassen den hadde da du gikk inn. Den blir stående
    /// der i delen til du går ut, så kortet ikke flytter seg for hver melding.
    låst: Option<(String, Plass)>,
    /// Prosjektene som er brettet sammen, når lista er ordnet etter mappe.
    brettet: HashSet<String>,

    /// Claude-en som er valgt i lista, og som vises til høyre.
    valgt: Option<Pane>,
    /// Pane-id-en til kortet piltastene står på i lista. Den byttes til først
    /// med `⏎`, så du kan bla forbi andre uten å bytte sesjon i tmux.
    peker: Option<String>,
    /// Bredden på lista med skillelinja, når den ikke er en stripe.
    liste_bredde: u16,
    /// Lista er en stripe med bare tegnene.
    liste_stripe: bool,
    drar_skille: bool,
    /// Første rad i lista som vises, når den ikke får plass.
    liste_rull: u16,
    /// Lista ruller så den valgte vises, til du ruller den selv.
    liste_følger: bool,
    /// Vinduet som vises til høyre: vinduet til den valgte, eller et annet
    /// vindu i samme sesjon.
    vist: Option<String>,
    forrige_vindu: Option<String>,
    /// Panene i vinduet, der tmux har plassert dem.
    vindu: Vec<Pane>,
    /// Pane-id → innholdet.
    bilder: HashMap<String, Vec<Linje>>,
    /// Resten gjelder den aktive panen i vinduet, den tastene går til.
    markør: Option<(u16, u16)>,
    /// Panen har slått på mus med SGR-koding, slik Claude Code gjør i fullskjerm.
    pane_mus: bool,
    /// Panen vil også ha musebevegelse uten knapp (modus 1003).
    pane_bevegelse: bool,
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
    størrelser: HashMap<String, (u16, u16)>,

    fokus: Fokus,
    prefiks: taster::Prefiks,
    utseende: Utseende,
    /// Utseende-menyen er åpen, med aksen som er markert.
    utseende_meny: Option<usize>,
    /// Prefikset er trykket, og neste tast er en kommando.
    etter_prefiks: bool,
    bindinger: bindinger::Bindinger,
    /// Til da går en tast med `-r` til tmux-kommandoen uten prefikset.
    gjentar: Option<Instant>,
    /// Det som lukkes hvis du svarer `y`.
    lukke: Option<Lukke>,
    visning: Rect,
    /// Kolonne og rad, til terminalen mister fokus. Terminalen sier ikke fra
    /// når musen forlater vinduet.
    musen: Option<(u16, u16)>,
    /// Det som sist ble klikket på, og når, så et klikk til er et dobbeltklikk.
    sist_klikket: Option<(Mål, Instant)>,
    /// Det forrige klikket gikk ikke til panen, og det gjør ikke slippet heller.
    svelg_slipp: bool,
    /// I den aktive panen, når den ikke har slått på mus selv.
    markering: Option<Markering>,
    /// Da markeringen sist ble kopiert, så det kan stå en liten stund.
    kopiert: Option<Instant>,
    /// Mappevelgeren, når den er oppe.
    velger: Option<ny::Velger>,
    handlinger: Option<Handlinger>,
    /// Hjelpen er åpen, rullet så mange linjer ned.
    hjelp: Option<u16>,
    /// Pane-id → mappe og starttid for Claude-er nerdr har startet, til de
    /// står i registeret. Oppstarten tar flere sekunder, og så lenge vises de
    /// likevel.
    nye: HashMap<String, (PathBuf, Instant)>,
    /// Panene tmux har fått fargene til terminalen for. En control mode-klient
    /// har ingen terminal tmux kan spørre, så uten dem svarer tmux svart når en
    /// pane spør om bakgrunnen, og Claude Code med temaet «auto» blir mørk også
    /// i en lys terminal.
    fargelagt: HashSet<String>,
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
        ["forklar"] if !json && fortsett.is_none() => kommandoer::skriv(&forklar::tekst(socket)),
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
    eprintln!(
        "bruk: nerdr [liste | samtaler | ny <mappe> | ny --fortsett <id> [mappe] | forklar] [--json] [-L socket], eller nerdr --help"
    );
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
        io::ErrorKind::NotFound => io::Error::other("fant ikke tmux, som nerdr trenger."),
        _ => feil,
    }
}

fn kjør(socket: Option<&str>) -> io::Result<()> {
    if kontroll::inne_i(socket) {
        return Err(io::Error::other("du er allerede inne i nerdr."));
    }
    tegn::bruk_farger(palett::les());
    let (tx, rx) = mpsc::channel();
    let socket = socket.map(String::from);
    let koble = {
        let (socket, tx) = (socket.clone(), tx.clone());
        move || {
            let tx = tx.clone();
            Kontroll::start(socket.as_deref(), move |m| {
                let _ = tx.send(Hendelse::Tmux(m));
            })
        }
    };
    let kontroll = koble()?;
    let koble_igjen = move || {
        // Uten sesjoner ville tilkoblingen laget en, og da er det du som har
        // avsluttet tmux.
        tmux::kjør(socket.as_deref(), &["has-session"])?;
        koble()
    };
    let prosjekter_tx = tx.clone();
    thread::spawn(move || {
        while let Ok(e) = event::read() {
            if tx.send(Hendelse::Term(e)).is_err() {
                break;
            }
        }
    });

    let mut terminal = ratatui::init();
    execute!(io::stdout(), EnableMouseCapture, EnableBracketedPaste, EnableFocusChange)?;
    let mut app = App::ny(kontroll, Box::new(koble_igjen), prosjekter_tx);
    let res = app.løkke(&mut terminal, &rx);
    let _ = execute!(io::stdout(), DisableMouseCapture, DisableBracketedPaste, DisableFocusChange);
    ratatui::restore();

    res?;
    match app.ferdig {
        Some(grunn) if !grunn.is_empty() => Err(io::Error::other(grunn)),
        _ => Ok(()),
    }
}

impl App {
    fn ny(kontroll: Kontroll, koble_igjen: Box<dyn FnMut() -> io::Result<Kontroll>>, tx: Sender<Hendelse>) -> App {
        App {
            kontroll,
            tx,
            koble_igjen,
            koblet_igjen: None,
            kø: VecDeque::new(),
            claude_paner: Vec::new(),
            alle: Vec::new(),
            lister: false,
            list_igjen: false,
            sist_listet: None,
            konfig: konfigkatalog(),
            claude: HashMap::new(),
            utenfor_tmux: 0,
            repoer: HashMap::new(),
            sett: HashMap::new(),
            låst: None,
            brettet: HashSet::new(),
            valgt: None,
            peker: None,
            liste_bredde: tegn::LISTE,
            liste_stripe: false,
            drar_skille: false,
            liste_rull: 0,
            liste_følger: true,
            vist: None,
            forrige_vindu: None,
            vindu: Vec::new(),
            bilder: HashMap::new(),
            markør: None,
            pane_mus: false,
            pane_bevegelse: false,
            alternativ: false,
            historie: 0,
            rull: 0,
            utdatert: false,
            henter: false,
            sist_hentet: Instant::now(),
            synk: HashMap::new(),
            output_hale: HashMap::new(),
            størrelser: HashMap::new(),
            fokus: Fokus::Liste,
            prefiks: taster::Prefiks::fra_miljø().unwrap_or_default(),
            utseende: Utseende::ny(bokser::Rammer::Linjer),
            utseende_meny: None,
            etter_prefiks: false,
            bindinger: bindinger::Bindinger::default(),
            gjentar: None,
            lukke: None,
            visning: Rect::default(),
            musen: None,
            sist_klikket: None,
            svelg_slipp: false,
            markering: None,
            kopiert: None,
            velger: None,
            handlinger: None,
            hjelp: None,
            nye: HashMap::new(),
            fargelagt: HashSet::new(),
            gå_inn_i: None,
            ferdig: None,
        }
    }

    fn løkke(&mut self, terminal: &mut DefaultTerminal, rx: &Receiver<Hendelse>) -> io::Result<()> {
        if taster::Prefiks::fra_miljø().is_none() {
            self.send("show-options -gv prefix", Svar::Prefiks);
        }
        self.send("list-keys -T prefix", Svar::Bindinger);
        self.send("show-options -gv repeat-time", Svar::Gjentatid);
        self.send(&format!("show-options -gqv {LISTE_OPTION}"), Svar::Liste);
        self.send(&format!("show-options -gqv {}", utseende::OPTION), Svar::Utseende);
        self.list_paner();
        let mut tegne = true;
        while self.ferdig.is_none() {
            if tegne {
                let liste = if self.liste_stripe { tegn::STRIPE } else { self.liste_bredde };
                self.visning = tegn::visning(terminal.size()?, liste);
                let rader = tegn::menyrader(self);
                if let Some(h) = &mut self.handlinger {
                    h.rull = tegn::følg(h.valgt, h.rull, rader, h.treff().len());
                }
                if let Some(v) = &mut self.velger {
                    v.rull = tegn::følg(v.valgt, v.rull, rader, v.treff().len());
                }
                let (utsnitt, valgt) = tegn::utsnitt(self);
                self.liste_rull = match valgt.filter(|_| self.liste_følger) {
                    Some(valgt) => utsnitt.følg(valgt).rull,
                    None => utsnitt.rull,
                };
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
            if self.kopiert.is_some_and(|t| t.elapsed() >= KOPIERT_VISES) {
                self.kopiert = None;
                tegne = true;
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

    /// Svarene på det som var sendt, kommer aldri, og den nye klienten står i
    /// sesjonen tmux velger. Kobles det fra igjen med en gang, gir nerdr opp.
    fn koble_til_igjen(&mut self) -> bool {
        if self.koblet_igjen.is_some_and(|t| t.elapsed() < Duration::from_secs(1)) {
            return false;
        }
        let Ok(kontroll) = (self.koble_igjen)() else { return false };
        self.kontroll = kontroll;
        self.koblet_igjen = Some(Instant::now());
        self.kø.clear();
        (self.lister, self.list_igjen, self.henter) = (false, false, false);
        self.størrelser.clear();
        self.fargelagt.clear();
        self.utdatert = true;
        if let Some(valgt) = self.valgt.clone() {
            self.send(&format!("switch-client -t {}", sitat(&valgt.sesjon_id)), Svar::Ingenting);
        }
        self.list_paner();
        true
    }

    /// Svarene terminalen ville gitt på OSC 10 og 11, som tmux gir panen når
    /// den spør. tmux sender også et temavarsel til programmer som har slått
    /// på modus 2031, så en Claude som kjører, bytter tema med en gang.
    fn gi_farger(&mut self, pane: &str) {
        let Some((fg, bg)) = tegn::farger() else { return };
        for (nr, (r, g, b)) in [(10, fg), (11, bg)] {
            let svar = format!("{pane}:\x1b]{nr};rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}\x07");
            self.send(&format!("refresh-client -r {}", dobbelt_sitat(&svar)), Svar::Ingenting);
        }
        self.fargelagt.insert(pane.to_string());
    }

    fn send(&mut self, kommando: &str, svar: Svar) {
        match self.kontroll.send(kommando) {
            Ok(()) => self.kø.push_back(svar),
            Err(feil) => self.ferdig = Some(format!("tmux: {feil}")),
        }
    }

    fn håndter(&mut self, h: Hendelse) -> bool {
        match h {
            Hendelse::Tmux(Melding::Svar { ok, linjer }) => match self.kø.pop_front() {
                Some(svar) => self.håndter_svar(svar, ok, linjer),
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
                if grunn.is_empty() && self.koble_til_igjen() {
                    return true;
                }
                self.ferdig = Some(if grunn.is_empty() { "tmux koblet fra".into() } else { grunn });
                false
            }
            Hendelse::Term(Event::Key(k)) if k.kind == KeyEventKind::Press => self.tast(k),
            Hendelse::Term(Event::Mouse(m)) => self.mus(m),
            Hendelse::Term(Event::Paste(tekst)) => match (&mut self.handlinger, &mut self.velger) {
                (Some(handlinger), _) => {
                    handlinger.skriv(&tekst);
                    true
                }
                (None, Some(velger)) => {
                    velger.skriv(&tekst);
                    true
                }
                (None, None) if self.lukke.is_some() => false,
                (None, None) => {
                    self.lim_inn(&tekst);
                    false
                }
            },
            Hendelse::Prosjekter(prosjekter) => match &mut self.velger {
                Some(velger) => {
                    velger.legg_til(prosjekter);
                    true
                }
                None => false,
            },
            Hendelse::Term(Event::FocusLost) => self.musen.take().is_some(),
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

    fn håndter_svar(&mut self, svar: Svar, ok: bool, linjer: Vec<String>) -> bool {
        match svar {
            Svar::Paner => {
                self.lister = false;
                self.sist_listet = Some(Instant::now());
                self.claude = register::les(&self.konfig.join("sessions"));
                let alle: Vec<Pane> = linjer.iter().filter_map(|l| Pane::fra_linje(l)).collect();
                let tom = !alle.iter().any(|p| self.claude.contains_key(&p.id));
                self.utenfor_tmux = if tom { register::utenfor_tmux(&self.konfig.join("sessions")) } else { 0 };
                self.husk_sett(&alle);
                if ok {
                    let inne = |id: &String| self.fokus == Fokus::Pane && self.valgt.as_ref().is_some_and(|v| &v.id == id);
                    self.nye.retain(|id, (_, startet)| {
                        !self.claude.contains_key(id)
                            && alle.iter().any(|p| &p.id == id)
                            && (startet.elapsed() < STARTER_MAKS || inne(id))
                    });
                    self.fargelagt.retain(|id| alle.iter().any(|p| &p.id == id));
                    for pane in &alle {
                        if !self.fargelagt.contains(&pane.id) {
                            self.gi_farger(&pane.id);
                        }
                    }
                }
                self.repoer = self
                    .claude
                    .iter()
                    .filter_map(|(id, s)| Some((id, s.mappe.as_deref()?)))
                    .chain(self.nye.iter().map(|(id, (m, _))| (id, m.as_path())))
                    .filter_map(|(id, mappe)| Some((id.clone(), git::les(mappe)?)))
                    .collect();
                self.claude_paner = alle
                    .iter()
                    .filter(|p| ok && (self.claude.contains_key(&p.id) || self.nye.contains_key(&p.id)))
                    .cloned()
                    .collect();
                self.størrelser.retain(|vindu, _| alle.iter().any(|p| &p.vindu_id == vindu));
                self.alle = alle;
                let valgt = self.valgt.as_ref().and_then(|v| self.claude_paner.iter().find(|p| p.id == v.id));
                match valgt.cloned() {
                    // Oppdatert, så størrelsen regnes fra det tmux sier nå.
                    Some(pane) => {
                        self.valgt = Some(pane);
                        self.oppdater_vindu(None);
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
                if !self.peker.as_ref().is_some_and(|id| self.claude_paner.iter().any(|p| &p.id == id)) {
                    self.peker = self.valgt.as_ref().map(|p| p.id.clone());
                }
                let ny = self.gå_inn_i.as_ref().and_then(|id| self.claude_paner.iter().find(|p| &p.id == id));
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
            Svar::Markør => {
                self.henter = false;
                let felt: Vec<&str> = linjer.first().map_or("", |l| l.as_str()).split('\t').collect();
                self.markør = match felt.as_slice() {
                    [x, y, "1", ..] => x.parse().ok().zip(y.parse().ok()),
                    _ => None,
                };
                self.pane_mus = felt.get(3) == Some(&"1");
                self.alternativ = felt.get(4) == Some(&"1");
                self.pane_bevegelse = felt.get(6) == Some(&"1");
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
            Svar::NySesjon { navn, mappe, claude_kommando } if !ok => {
                self.send(&tmux::linje(&start::nytt_vindu(&navn, &mappe)), Svar::NyPane { mappe, claude_kommando });
                false
            }
            Svar::NySesjon { mappe, claude_kommando, .. } | Svar::NyPane { mappe, claude_kommando } => {
                let Some(id) = linjer.into_iter().next().filter(|_| ok) else { return false };
                // Før `claude` skrives inn, så den finner temaet når den starter.
                self.gi_farger(&id);
                for kommando in start::skriv_inn(&id, &claude_kommando) {
                    self.send(&tmux::linje(&kommando), Svar::Ingenting);
                }
                self.nye.insert(id.clone(), (mappe, Instant::now()));
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
                    self.bindinger = bindinger::Bindinger::ny(&nerdr_kjerne::bindinger::les(&linjer));
                    self.bindinger.gjentatid = gjentatid;
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

    /// Tasten etter prefikset, med bindingene fra tmux-oppsettet ditt.
    fn etter_prefikset(&mut self, k: KeyEvent) {
        match self.bindinger.finn(&k).cloned() {
            _ if self.prefiks.er(&k) => self.prefikstast(k),
            Some(bindinger::Hva::Som(nerdr)) => self.prefikstast(nerdr),
            Some(bindinger::Hva::Send(kommandoer)) => {
                for kommando in kommandoer {
                    self.send(&kommando, Svar::Ingenting);
                }
            }
            None => self.prefikstast(k),
        }
        if self.bindinger.gjentas(&k) {
            self.gjentar = Some(Instant::now() + self.bindinger.gjentatid);
        }
    }

    /// Tasten nerdr har etter prefikset, som er den samme som i tmux uten
    /// noe oppsett. Virker både fra lista og fra panen.
    fn prefikstast(&mut self, k: KeyEvent) {
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
            KeyCode::Char('b' | 'B') if !ctrl => {
                self.utseende_meny = Some(1);
                return;
            }
            KeyCode::Char('?') => return self.hjelp = Some(0),
            KeyCode::Char(c @ ('f' | 'r')) if !ctrl => return self.ny_sesjon(c == 'r'),
            KeyCode::Char(':') => return self.åpne_handlinger(),
            _ => {}
        }
        let (Some(aktiv), Some(vindu)) = (self.aktiv().cloned(), self.vist.as_ref().map(|v| sitat(v))) else {
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
            (KeyCode::Char('c'), _) => {
                let sesjon = sitat(&format!("{}:", aktiv.sesjon_id));
                self.send(&format!("new-window -t {sesjon} -c '#{{pane_current_path}}' -P -F '#{{window_id}}'"), Svar::NyttVindu);
                return;
            }
            (KeyCode::Char('x'), _) => format!("kill-pane -t {id}"),
            (KeyCode::Char('&'), _) => format!("kill-window -t {}", sitat(&aktiv.vindu_id)),
            (KeyCode::PageUp, _) => {
                let side = self.bokser().iter().find(|b| b.pane == aktiv.id).map_or(1, |b| b.innhold.height.max(1));
                return self.rull_til(self.rull.saturating_add(side as u32));
            }
            _ => return,
        };
        self.send(&kommando, Svar::Ingenting);
    }

    /// Neste eller forrige Claude i lista, som `(` og `)` bytter sesjon i
    /// tmux. Fra panen går du inn i den.
    fn bla(&mut self, steg: isize) {
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
    fn faner(&self) -> Vec<&Pane> {
        let Some(sesjon) = self.vindu.first().map(|p| &p.sesjon_id) else { return Vec::new() };
        let mut sett = HashSet::new();
        let mut faner: Vec<&Pane> =
            self.alle.iter().filter(|p| &p.sesjon_id == sesjon && sett.insert(&p.vindu_id)).collect();
        faner.sort_by_key(|p| p.vindu.parse::<u32>().unwrap_or(u32::MAX));
        faner
    }

    fn neste_vindu(&mut self, steg: isize) {
        let faner: Vec<String> = self.faner().iter().map(|p| p.vindu_id.clone()).collect();
        let Some(i) = faner.iter().position(|v| Some(v) == self.vist.as_ref()) else { return };
        let neste = (i as isize + steg).rem_euclid(faner.len() as isize) as usize;
        self.vis_vindu(faner[neste].clone());
    }

    /// Viser et annet vindu. Står det en Claude i det, blir den valgt i lista.
    fn vis_vindu(&mut self, vindu: String) {
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

    fn vis(&mut self, vindu: String, aktiv: Option<&str>) {
        if self.vist.as_ref() != Some(&vindu) {
            self.forrige_vindu = self.vist.replace(vindu);
        }
        self.bilder.clear();
        self.synk.clear();
        self.output_hale.clear();
        self.oppdater_vindu(aktiv);
        self.nullstill_aktiv();
    }

    /// Samme kommandoer som `start::start`, men gjennom control mode, så svaret
    /// kommer som en melding.
    fn start_claude(&mut self, mappe: PathBuf, samtale: Option<&str>) {
        let navn = start::sesjonsnavn(&mappe);
        let claude_kommando = start::claude(samtale);
        self.send(&tmux::linje(&start::ny_sesjon(&navn, &mappe)), Svar::NySesjon { navn, mappe, claude_kommando });
    }

    /// Er en liste alt på vei, kan den være bestilt før endringen, så det
    /// listes en gang til når den kommer.
    fn list_paner(&mut self) {
        if self.lister {
            self.list_igjen = true;
        } else {
            self.lister = true;
            self.send(&format!("list-panes -a -F {}", sitat(FELT)), Svar::Paner);
        }
    }

    fn velg(&mut self, pane: Pane) {
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
    fn oppdater_vindu(&mut self, aktiv: Option<&str>) {
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
    fn bokser(&self) -> Vec<Boks> {
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

    fn oppsett(&self) -> Option<oppsett::Celle> {
        oppsett::tolk(&self.vindu.first()?.oppsett)
    }

    /// Den aktive panen i vinduet, eller den valgte før vinduet dens er
    /// listet. Et nytt vindu har ingen før det er listet, så tastene ikke går
    /// til Claude-en i et annet vindu.
    fn aktiv(&self) -> Option<&Pane> {
        let valgt = self.valgt.as_ref().filter(|v| Some(&v.vindu_id) == self.vist.as_ref());
        self.vindu.iter().find(|p| p.aktiv).or(valgt)
    }

    fn aktiver(&mut self, id: &str) {
        self.send(&format!("select-pane -t {}", sitat(id)), Svar::Ingenting);
        for p in &mut self.vindu {
            p.aktiv = p.id == id;
        }
        self.nullstill_aktiv();
    }

    fn nullstill_aktiv(&mut self) {
        self.markering = None;
        self.markør = None;
        self.pane_mus = false;
        self.pane_bevegelse = false;
        self.alternativ = false;
        self.historie = 0;
        self.rull = 0;
        self.utdatert = true;
    }

    fn hent(&mut self) {
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
            let fra = if id == aktiv && self.rull > 0 { format!(" -S -{}", self.rull) } else { String::new() };
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
    fn synk_størrelse(&mut self) {
        if let Some(vist) = self.vist.clone().filter(|_| !self.vindu.is_empty()) {
            self.synk_størrelse_for(&vist);
        }
    }

    fn synk_størrelse_for(&mut self, vindu: &str) {
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

    fn tast(&mut self, k: KeyEvent) -> bool {
        if self.hjelp.is_some() {
            let steg = match k.code {
                KeyCode::Up | KeyCode::Char('k') => -1,
                KeyCode::Down | KeyCode::Char('j') => 1,
                _ => 0,
            };
            // Får hjelpen plass, lukker hver tast den, som hintet sier.
            if steg == 0 || tegn::hjelp_rull_maks(self) == 0 {
                self.hjelp = None;
            } else {
                self.rull_hjelp(steg);
            }
            return true;
        }
        if let Some(akse) = self.utseende_meny {
            let steg = match k.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                    self.utseende_meny = None;
                    return true;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.utseende_meny = Some(self.utseende.nabo(akse, -1));
                    return true;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.utseende_meny = Some(self.utseende.nabo(akse, 1));
                    return true;
                }
                KeyCode::Left | KeyCode::Char('h') => -1,
                KeyCode::Right | KeyCode::Char('l' | ' ') => 1,
                _ => return false,
            };
            self.utseende.endre(akse, steg);
            self.lagre_utseende();
            return true;
        }
        let side = tegn::menyrader(self) as isize;
        if let Some(handlinger) = &mut self.handlinger {
            let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
            match k.code {
                KeyCode::Esc => self.handlinger = None,
                KeyCode::Char('c') if ctrl => self.handlinger = None,
                KeyCode::Enter => self.kjør_valgt_handling(),
                KeyCode::PageUp => handlinger.flytt(-side),
                KeyCode::PageDown => handlinger.flytt(side),
                KeyCode::Up => handlinger.flytt(-1),
                KeyCode::Char('p' | 'k') if ctrl => handlinger.flytt(-1),
                KeyCode::Down => handlinger.flytt(1),
                KeyCode::Char('n' | 'j') if ctrl => handlinger.flytt(1),
                KeyCode::Char('u') if ctrl => handlinger.tøm(),
                KeyCode::Backspace => handlinger.slett(),
                KeyCode::Char(c) if !ctrl => handlinger.skriv(c.encode_utf8(&mut [0; 4])),
                _ => return false,
            }
            return true;
        }
        if let Some(velger) = &mut self.velger {
            let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
            match k.code {
                KeyCode::Esc => self.velger = None,
                KeyCode::Char('c') if ctrl => self.velger = None,
                KeyCode::Enter => self.start_valgt(),
                KeyCode::PageUp => velger.flytt(-side),
                KeyCode::PageDown => velger.flytt(side),
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
        if let Some(lukke) = self.lukke.take() {
            if k.code == KeyCode::Char('y') {
                self.send(&lukke.kommando, Svar::Ingenting);
            }
            return true;
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
        if self.fokus == Fokus::Pane {
            if let (Some(pane), Some(args)) = (self.aktiv(), taster::send_keys(&k)) {
                let kommando = format!("send-keys -t {} {args}", sitat(&pane.id));
                self.send(&kommando, Svar::Ingenting);
                self.rull_til(0);
            }
            return self.markering.take().is_some();
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

    /// Prosjektet til kortet som er markert, når lista er ordnet etter mappe.
    fn valgt_prosjekt(&self) -> Option<String> {
        let id = self.markert().filter(|_| self.utseende.orden == Orden::Mappe)?;
        self.claude_paner.iter().find(|p| p.id == id).map(|p| self.prosjekt(p))
    }

    fn brett(&mut self, sammen: bool) -> bool {
        let Some(prosjekt) = self.valgt_prosjekt() else { return false };
        if sammen { self.brettet.insert(prosjekt) } else { self.brettet.remove(&prosjekt) }
    }

    fn kjør_valgt_handling(&mut self) {
        if let Some(handling) = self.handlinger.take().and_then(|h| h.valgt()) {
            self.kjør(handling);
        }
    }

    /// Uten treff står velgeren åpen, så du kan søke på nytt.
    fn start_valgt(&mut self) {
        let Some((mappe, samtale)) = self.velger.as_ref().and_then(|v| v.valgt()).map(|k| (k.mappe.clone(), k.samtale.clone()))
        else {
            return;
        };
        self.velger = None;
        self.start_claude(mappe, samtale.as_deref());
    }

    fn flytt_i_menyen(&mut self, steg: isize) {
        if let Some(h) = &mut self.handlinger {
            h.flytt(steg);
        } else if let Some(v) = &mut self.velger {
            v.flytt(steg);
        }
    }

    fn åpne_handlinger(&mut self) {
        self.handlinger = Some(Handlinger::ny(&self.prefiks.navn(), &self.bindinger, &self.utseende));
    }

    fn kjør(&mut self, handling: Handling) {
        match handling {
            Handling::Prefiks(kode, ctrl) => {
                let m = if ctrl { KeyModifiers::CONTROL } else { KeyModifiers::NONE };
                self.prefikstast(KeyEvent::new(kode, m));
            }
            Handling::Utseende(akse, i) => {
                self.utseende.sett(akse, i);
                self.lagre_utseende();
            }
        }
    }

    fn lagre_utseende(&mut self) {
        // Rammene endrer hvor mye plass tmux får.
        self.utdatert = true;
        self.send(&format!("set-option -g {} {}", utseende::OPTION, sitat(&self.utseende.tekst())), Svar::Ingenting);
    }

    /// «ny sesjon» lager et vindu til Claude-en, og det er vinduet som lukkes,
    /// med det du har åpnet ved siden av. Et skall som står igjen etter
    /// Claude-en, viser ikke nerdr. Deler flere Claude-er vinduet, lukkes bare
    /// panen.
    fn lukk_sesjon(&mut self, pane: &Pane) {
        let claude = |id: &String| self.claude.contains_key(id) || self.nye.contains_key(id);
        let flere = self.alle.iter().any(|p| p.vindu_id == pane.vindu_id && p.id != pane.id && claude(&p.id));
        let tittel = pane.oppgave().map_or_else(|| self.sted(pane), str::to_string);
        let (hva, kommando) = match flere {
            true => (format!("«{tittel}». De andre Claude-ene i vinduet står igjen."), format!("kill-pane -t {}", sitat(&pane.id))),
            false => (format!("«{tittel}» og vinduet den står i."), format!("kill-window -t {}", sitat(&pane.vindu_id))),
        };
        self.lukke = Some(Lukke { spørsmål: "lukke sesjonen?", hva, kommando });
    }

    /// Det som starter en Claude, går via lista, så du går inn i den nye selv
    /// om du startet den fra en pane.
    fn ny_sesjon(&mut self, fortsett: bool) {
        self.bytt_fokus(Fokus::Liste);
        self.åpne_velger(fortsett);
    }

    /// Prosjektene under mappene tar tid å lese i store repoer, så velgeren
    /// viser mappene først og får prosjektene når de er lest.
    fn åpne_velger(&mut self, fortsett: bool) {
        let kjører: HashSet<String> = self.claude.values().filter_map(|s| s.samtale.clone()).collect();
        let velger = ny::Velger::ny(fortsett, &self.konfig.join("projects"), &kjører);
        let (mapper, tx) = (velger.toppmapper(), self.tx.clone());
        thread::spawn(move || {
            let _ = tx.send(Hendelse::Prosjekter(ny::prosjekter(&mapper)));
        });
        self.velger = Some(velger);
    }

    /// `paste-buffer -p` legger på markørene for bracketed paste bare når panen
    /// har bedt om dem, og uten dem sender Claude inn ved første linjeskift.
    fn lim_inn(&mut self, tekst: &str) {
        let Some(pane) = self.aktiv().filter(|_| self.fokus == Fokus::Pane) else { return };
        let id = sitat(&pane.id);
        // paste-buffer gjør LF om til CR, så CRLF ville blitt to linjeskift.
        let tekst = tekst.replace("\r\n", "\n");
        self.send(&format!("set-buffer -b nerdr {}", dobbelt_sitat(&tekst)), Svar::Ingenting);
        self.send(&format!("paste-buffer -p -d -b nerdr -t {id}"), Svar::Ingenting);
        self.rull_til(0);
    }

    fn rull_til(&mut self, rull: u32) {
        let rull = rull.min(self.historie);
        if rull != self.rull {
            self.rull = rull;
            self.utdatert = true;
        }
    }

    fn rull_hjelp(&mut self, steg: i16) {
        let maks = tegn::hjelp_rull_maks(self);
        if let Some(rull) = &mut self.hjelp {
            *rull = rull.saturating_add_signed(steg).min(maks);
        }
    }

    fn mus(&mut self, m: MouseEvent) -> bool {
        let før = self.under_musen();
        self.musen = Some((m.column, m.row));
        let flyttet = self.under_musen() != før;
        // Et nytt klikk fjerner markeringen, også når det starter en ny.
        let fjernet = matches!(m.kind, MouseEventKind::Down(_)) && self.markering.take().is_some();
        self.håndter_mus(m) || flyttet || fjernet
    }

    /// Regnes ut på nytt hver gang, så det følger med når lista flytter seg
    /// under en mus som står stille.
    fn under_musen(&self) -> Option<Mål> {
        let (x, y) = self.musen?;
        if self.hjelp.is_some() {
            return None;
        }
        if self.utseende_meny.is_some() {
            return tegn::verdi_på(self, x, y).map(|(akse, i)| Mål::Verdi(akse, i));
        }
        if self.velger.is_some() || self.handlinger.is_some() {
            return tegn::meny_rad_på(self, x, y).map(Mål::Rad);
        }
        if self.lukke.is_some() {
            return tegn::svar_på(self, x, y).map(Mål::Svar);
        }
        let v = self.visning;
        if x + 1 == v.x {
            return Some(Mål::Skille);
        }
        if x < v.x && let Some(knapp) = tegn::knapp_på(self, x, y) {
            return Some(Mål::Knapp(knapp));
        }
        if x < v.x && tegn::sortering_på(self, x, y) {
            return Some(Mål::Sortering);
        }
        if x < v.x {
            if let Some(p) = tegn::lukk_på(self, x, y) {
                return Some(Mål::Lukk(p.id));
            }
            return tegn::pane_på(self, y).map(|p| Mål::Kort(p.id));
        }
        if y < v.y {
            if let Some(vindu) = tegn::lukk_fane_på(self, x) {
                return Some(Mål::LukkFane(vindu));
            }
            let ny = tegn::ny_fane_på(self, x).then_some(Mål::NyFane);
            return tegn::fane_på(self, x).map(Mål::Fane).or(ny);
        }
        let inni = |r: Rect| (r.left()..r.right()).contains(&x) && (r.top()..r.bottom()).contains(&y);
        let boks = self.bokser().into_iter().find(|b| inni(b.ramme.unwrap_or(b.innhold)))?;
        let aktiv = self.aktiv().is_some_and(|p| p.id == boks.pane);
        (self.fokus == Fokus::Liste || !aktiv).then_some(Mål::Pane(boks.pane))
    }

    fn håndter_mus(&mut self, m: MouseEvent) -> bool {
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
        if self.hjelp.is_some() {
            match m.kind {
                MouseEventKind::ScrollUp => self.rull_hjelp(-(RULL_STEG as i16)),
                MouseEventKind::ScrollDown => self.rull_hjelp(RULL_STEG as i16),
                _ => return false,
            }
            return true;
        }
        if self.velger.is_some() || self.handlinger.is_some() {
            let (x, y) = (m.column, m.row);
            match m.kind {
                MouseEventKind::ScrollUp => self.flytt_i_menyen(-1),
                MouseEventKind::ScrollDown => self.flytt_i_menyen(1),
                // Et klikk utenfor lukker, som Esc.
                MouseEventKind::Down(MouseButton::Left) => match tegn::meny_rad_på(self, x, y) {
                    Some(i) => {
                        if let Some(h) = &mut self.handlinger {
                            h.valgt = i;
                            self.kjør_valgt_handling();
                        } else if let Some(v) = &mut self.velger {
                            v.valgt = i;
                            self.start_valgt();
                        }
                    }
                    None if tegn::i_menyboks(self, x, y) => {}
                    None => (self.handlinger, self.velger) = (None, None),
                },
                _ => return false,
            }
            return true;
        }
        if self.utseende_meny.is_some() {
            if m.kind != MouseEventKind::Down(MouseButton::Left) {
                return false;
            }
            match tegn::verdi_på(self, m.column, m.row) {
                Some((akse, i)) => {
                    self.utseende_meny = Some(akse);
                    self.utseende.sett(akse, i);
                    self.lagre_utseende();
                }
                None if tegn::i_valgmenyen(self, m.column, m.row) => {}
                None => self.utseende_meny = None,
            }
            return true;
        }
        if self.lukke.is_some() {
            if m.kind != MouseEventKind::Down(MouseButton::Left) {
                return false;
            }
            // Et klikk utenfor popupen avbryter, som en tast som ikke er y.
            match tegn::svar_på(self, m.column, m.row) {
                Some(true) => {
                    if let Some(lukke) = self.lukke.take() {
                        self.send(&lukke.kommando, Svar::Ingenting);
                    }
                }
                None if tegn::i_popupen(self, m.column, m.row) => {}
                _ => self.lukke = None,
            }
            return true;
        }
        if self.markering.as_ref().is_some_and(|m| m.drar)
            && matches!(m.kind, MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left))
        {
            return self.dra_markering(m);
        }
        let v = self.visning;
        if m.column + 1 == v.x && m.kind == MouseEventKind::Down(MouseButton::Left) {
            let dobbelt = self.sist_klikket.take().is_some_and(|(mål, t)| mål == Mål::Skille && t.elapsed() < DOBBELTKLIKK);
            if dobbelt {
                self.liste_stripe = !self.liste_stripe;
                self.lagre_liste();
            } else {
                self.sist_klikket = Some((Mål::Skille, Instant::now()));
                self.drar_skille = true;
            }
            return true;
        }
        if m.row < v.y && m.column >= v.x {
            if m.row != 0 || m.kind != MouseEventKind::Down(MouseButton::Left) {
                return false;
            }
            if let Some(vindu) = tegn::lukk_fane_på(self, m.column) {
                self.send(&format!("kill-window -t {}", sitat(&vindu)), Svar::Ingenting);
            } else if let Some(vindu) = tegn::fane_på(self, m.column) {
                self.vis_vindu(vindu);
            } else if tegn::ny_fane_på(self, m.column) {
                self.prefikstast(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
            }
            return true;
        }
        if m.column < v.x
            && let Some(knapp) = tegn::knapp_på(self, m.column, m.row)
        {
            if m.kind != MouseEventKind::Down(MouseButton::Left) {
                return false;
            }
            self.prefikstast(KeyEvent::new(KeyCode::Char(tegn::tasten_til(knapp)), KeyModifiers::NONE));
            return true;
        }
        if m.column < v.x && tegn::sortering_på(self, m.column, m.row) {
            if m.kind != MouseEventKind::Down(MouseButton::Left) {
                return false;
            }
            self.utseende.endre(utseende::ORDEN, 1);
            self.lagre_utseende();
            return true;
        }
        if m.column < v.x {
            let (utsnitt, _) = tegn::utsnitt(self);
            let steg = match m.kind {
                MouseEventKind::ScrollUp => Some(-(RULL_STEG as i32)),
                MouseEventKind::ScrollDown => Some(RULL_STEG as i32),
                MouseEventKind::Down(MouseButton::Left) => tegn::pil_på(self, m.row).map(|opp| {
                    let side = utsnitt.rom.max(1) as i32;
                    if opp { -side } else { side }
                }),
                _ => None,
            };
            if let Some(steg) = steg.filter(|_| utsnitt.ruller) {
                // Tegningen holder rullet innenfor lista.
                self.liste_rull = (self.liste_rull as i32 + steg).max(0) as u16;
                self.liste_følger = false;
                return true;
            }
            if m.kind != MouseEventKind::Down(MouseButton::Left) {
                return false;
            }
            if let Some(pane) = tegn::lukk_på(self, m.column, m.row) {
                self.lukk_sesjon(&pane);
                return true;
            }
            if let Some(prosjekt) = tegn::prosjekt_på(self, m.row) {
                if !self.brettet.remove(&prosjekt) {
                    self.brettet.insert(prosjekt);
                }
                return true;
            }
            let ute = self.fokus == Fokus::Pane;
            // Kortet slås opp før låsen tas bort, ellers kan kortene ha flyttet
            // seg under klikket.
            let pane = tegn::pane_på(self, m.row);
            // Via lista, så panen du forlater, merkes som sett før du går inn.
            self.bytt_fokus(Fokus::Liste);
            let Some(pane) = pane else { return ute };
            let kort = Mål::Kort(pane.id.clone());
            let dobbelt = self.sist_klikket.take().is_some_and(|(mål, t)| mål == kort && t.elapsed() < DOBBELTKLIKK);
            self.peker = Some(pane.id.clone());
            if dobbelt {
                self.gå_inn();
            } else {
                self.sist_klikket = Some((kort, Instant::now()));
            }
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
        // Klikket som tar deg inn, går ikke videre, så det ikke trykker på et
        // valg i en dialog.
        if byttet {
            return true;
        }
        self.svelg_slipp = false;
        let (id, x, y) = (sitat(&pane.id), m.column - boks.innhold.x, m.row - boks.innhold.y);
        if !self.pane_mus && m.kind == MouseEventKind::Down(MouseButton::Left) {
            self.markering = Some(Markering::ny(pane.id, self.punkt(x, y)));
            return true;
        }
        let opp = match m.kind {
            MouseEventKind::ScrollUp => Some(true),
            MouseEventKind::ScrollDown => Some(false),
            _ => None,
        };
        match opp {
            _ if m.kind == MouseEventKind::Moved && !self.pane_bevegelse => {}
            _ if self.pane_mus => {
                if let Some(args) = taster::mus(&m, x, y) {
                    self.send(&format!("send-keys -t {id} {args}"), Svar::Ingenting);
                }
            }
            Some(opp) if self.alternativ => {
                let tast = if opp { "Up" } else { "Down" };
                self.send(&format!("send-keys -t {id} -N {RULL_STEG} {tast}"), Svar::Ingenting);
            }
            Some(true) => self.rull_til(self.rull.saturating_add(RULL_STEG)),
            Some(false) => self.rull_til(self.rull.saturating_sub(RULL_STEG)),
            None => {}
        }
        byttet
    }

    /// Markeringen følger musen også utenfor panen, men stopper ved kanten.
    fn dra_markering(&mut self, m: MouseEvent) -> bool {
        let pane = self.markering.as_ref().map(|m| m.pane.clone());
        let Some(boks) = self.bokser().into_iter().find(|b| Some(&b.pane) == pane.as_ref()) else {
            self.markering = None;
            return true;
        };
        let i = boks.innhold;
        let x = m.column.clamp(i.x, i.right().saturating_sub(1)) - i.x;
        let y = m.row.clamp(i.y, i.bottom().saturating_sub(1)) - i.y;
        let punkt = self.punkt(x, y);
        let Some(markering) = &mut self.markering else { return false };
        markering.dra(punkt);
        if m.kind != MouseEventKind::Up(MouseButton::Left) {
            return true;
        }
        markering.drar = false;
        if markering.tom() {
            self.markering = None;
            return true;
        }
        let topp = self.punkt(0, 0).0;
        if let (Some(markering), Some(bilde)) = (&self.markering, self.bilder.get(&boks.pane))
            && markering::kopier(&markering.tekst(bilde, topp, i.width))
        {
            self.kopiert = Some(Instant::now());
        }
        true
    }

    /// Streken følger musen. Drar du den forbi `LISTE_MIN`, blir lista en
    /// stripe, og drar du den ut igjen, får den bredden der musen er.
    fn dra_skille(&mut self, x: u16) {
        let bredde = x + 1;
        if bredde != self.visning.x {
            // Et klikk på streken etter å ha dratt den, er ikke et dobbeltklikk.
            self.sist_klikket = None;
        }
        self.liste_stripe = bredde < (tegn::STRIPE + tegn::LISTE_MIN) / 2;
        if !self.liste_stripe {
            self.liste_bredde = bredde.max(tegn::LISTE_MIN).min(self.visning.right() / 2);
        }
    }

    fn lagre_liste(&mut self) {
        let verdi = format!("{}{}", self.liste_bredde, if self.liste_stripe { " stripe" } else { "" });
        self.send(&format!("set-option -g {LISTE_OPTION} {}", sitat(&verdi)), Svar::Ingenting);
    }

    /// Der `x` og `y` i den aktive panen er, med linja regnet fra toppen av
    /// historikken.
    fn punkt(&self, x: u16, y: u16) -> Punkt {
        (self.historie as i64 - self.rull as i64 + y as i64, x)
    }

    /// Flytter pekeren, ikke det som vises.
    fn flytt(&mut self, steg: isize) {
        let fra = self.peker.clone();
        if let Some(neste) = self.nabo(fra.as_deref(), steg) {
            self.peker = Some(neste.id);
            self.liste_følger = true;
        }
    }

    /// Claude-en `steg` kort fra `fra` i lista, stoppet ved endene.
    fn nabo(&self, fra: Option<&str>, steg: isize) -> Option<Pane> {
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
    fn markert(&self) -> Option<&str> {
        match self.fokus {
            Fokus::Liste => self.peker.as_deref(),
            Fokus::Pane => self.valgt.as_ref().map(|p| p.id.as_str()),
        }
    }

    /// Velger kortet pekeren står på, og går inn i det, også når et annet
    /// vindu i sesjonen vises, som `choose-tree` i tmux.
    fn gå_inn(&mut self) {
        let pekt = self.peker.as_ref().and_then(|id| self.claude_paner.iter().find(|p| &p.id == id)).cloned();
        if let Some(pane) = pekt {
            let vises = self.valgt.as_ref().is_some_and(|v| v.id == pane.id) && Some(&pane.vindu_id) == self.vist.as_ref();
            if !vises {
                self.velg(pane);
            }
        }
        self.bytt_fokus(Fokus::Pane);
    }

    fn bytt_fokus(&mut self, fokus: Fokus) {
        if fokus == self.fokus {
            return;
        }
        self.låst = match fokus {
            Fokus::Pane => self.valgt.as_ref().and_then(|p| Some((p.id.clone(), self.plass(p)?))),
            Fokus::Liste => None,
        };
        self.peker = self.valgt.as_ref().map(|p| p.id.clone());
        self.liste_følger = true;
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

    fn merke(&self, pane: &Pane) -> Option<Merke> {
        let Some(claude) = self.claude.get(&pane.id) else {
            return self.nye.contains_key(&pane.id).then_some(Merke::Starter);
        };
        Some(Merke::fra(claude, self.sett.get(&pane.id).copied().unwrap_or(0)))
    }

    fn plass(&self, pane: &Pane) -> Option<Plass> {
        let merke = self.merke(pane)?;
        let del = Del::fra(merke);
        // Som tiden på kortet, så de ledige ikke flytter seg når du har vært inne i en.
        let siden = self.claude.get(&pane.id).map(|c| c.endret);
        Some(match &self.låst {
            // Den du er inne i, blir ikke ledig før du går ut. En ny Claude
            // regnes som sett fra den dukker opp i registeret. Settes en
            // ledig i gang, havner den øverst blant de aktive.
            Some((id, låst)) if *id == pane.id => {
                let del = del.min(låst.del);
                Plass { del, merke, siden: if del == låst.del { låst.siden } else { None } }
            }
            _ => Plass { del, merke, siden },
        })
    }

    /// Tar inn det tmux har for panene der en Claude kjører. En pane som
    /// ingen nerdr har sett før, får tiden nå, også i tmux, så det den blir
    /// ferdig med mens nerdr er lukket, venter på deg neste gang.
    fn husk_sett(&mut self, alle: &[Pane]) {
        self.sett.retain(|id, _| self.claude.contains_key(id));
        let nå = nå_ms();
        for pane in alle {
            if !self.claude.contains_key(&pane.id) {
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

    fn merk_sett(&mut self, id: &str, tid: u64) {
        self.sett.insert(id.to_string(), tid);
        self.send(&format!("set-option -p -t {} {} {tid}", sitat(id), paner::SETT), Svar::Ingenting);
    }

    fn rader(&self) -> Vec<Rad<'_>> {
        let paner = &self.claude_paner;
        match self.utseende.orden {
            Orden::Nerdr => grupper::rader(paner, |p| self.plass(p)),
            Orden::Mappe => {
                grupper::etter_mappe(paner, &self.brettet, |p| Some((self.plass(p)?.merke, self.prosjekt(p))))
            }
            Orden::Aktivitet => grupper::etter_aktivitet(paner, |p| self.plass(p)),
        }
    }

    /// Repoet panen jobber i, eller mappa når den ikke er i et repo.
    fn sted(&self, pane: &Pane) -> String {
        tekst::sted(pane, self.mappe(pane), self.repoer.get(&pane.id))
    }

    fn mappe(&self, pane: &Pane) -> Option<&std::path::Path> {
        let ny = self.nye.get(&pane.id).map(|(mappe, _)| mappe.as_path());
        self.claude.get(&pane.id).and_then(|c| c.mappe.as_deref()).or(ny)
    }

    /// Stien til repoet panen jobber i, så et worktree hører til repoet det
    /// er laget fra, og to repoer med samme navn er to prosjekter.
    fn prosjekt(&self, pane: &Pane) -> String {
        match (self.repoer.get(&pane.id), self.mappe(pane)) {
            (Some(repo), _) => tekst::med_tilde(&repo.rot),
            (None, Some(mappe)) => tekst::med_tilde(mappe),
            (None, None) => self.sted(pane),
        }
    }

    /// Hvor panen jobber, når det ikke er i mappa til prosjektet: stien inne
    /// i repoet, eller hele stien til et worktree.
    fn undermappe(&self, pane: &Pane) -> Option<String> {
        let (repo, mappe) = (self.repoer.get(&pane.id)?, self.mappe(pane)?);
        if mappe == repo.rot {
            return None;
        }
        Some(mappe.strip_prefix(&repo.rot).map_or_else(|_| tekst::med_tilde(mappe), |inne| inne.display().to_string()))
    }

    fn i_rekkefølge(&self) -> Vec<Pane> {
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

fn nå_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}
