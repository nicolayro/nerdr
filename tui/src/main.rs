mod agent;
mod app;
mod tmuxtaster;
mod bokser;
mod forklar;
mod grupper;
mod handlinger;
mod hjelp;
mod kommandoer;
mod kopimodus;
mod markering;
mod ny;
mod omverden;
mod palett;
mod popup;
mod taster;
#[cfg(test)]
mod tester;
mod tegn;
mod tekst;
mod utseende;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, io, thread};

use app::App;
use kopimodus::Kopimodus;
use nerdr_kjerne::kontroll::{self, Kontroll, Melding};
use omverden::Tmux;
use nerdr_kjerne::{konfigkatalog, start, tmux};
use markering::Markering;
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture, Event,
};
use ratatui::crossterm::execute;

/// En Claude nerdr har startet, som ikke står i registeret etter så lang tid,
/// kom trolig aldri i gang, som når `claude` ikke finnes i skallet. Den som
/// starter tregt, er i registeret lenge før.
const STARTER_MAKS: Duration = Duration::from_secs(5 * 60);
/// Så lenge en sesjon nerdr har laget, er uten Claude før den ryddes. Et
/// skall som er tregt å starte, rekker å starte `claude`, og et registeret som
/// leses midt i en skriving, lukker ingenting.
const UTEN_CLAUDE_MAKS: Duration = Duration::from_secs(10);

enum Hendelse {
    /// Med nummeret til tilkoblingen meldingen kom fra.
    Tmux(u64, Melding),
    Term(Event),
    /// Mappene til mappevelgeren, lest i en egen tråd, og om de er de siste.
    Mapper(Arc<AtomicBool>, Vec<PathBuf>, bool),
}

/// Hva neste svar fra tmux gjelder. Svarene kommer i samme rekkefølge som
/// kommandoene, så køen holder rede på dem.
enum Svar {
    Paner,
    /// Innholdet i panen med id-en.
    Bilde(String),
    Markør,
    /// Linjene fra `fra` og ned, i en pane som er `bredde` bred.
    Kopi { markering: Markering, fra: i64, bredde: u16 },
    /// Svaret er id-en til vinduet.
    NyttVindu,
    /// Svaret er pane-id-en til den nye sesjonen, eller en feil når navnet er
    /// tatt. `navn` er navnet til mappa, uten nummeret fra `forsøk`.
    NySesjon { navn: String, forsøk: u32, mappe: PathBuf, claude_kommando: String },
    Prefiks,
    /// Prefikstabellen fra `list-keys`.
    Bindinger,
    Root,
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

/// Det nerdr vet om den aktive panen. Byttes panen, er alt her gammelt, så
/// hele structen nullstilles samlet.
#[derive(Default)]
struct AktivPane {
    markør: Option<(u16, u16)>,
    /// Panen har slått på mus med SGR-koding, slik Claude Code gjør i fullskjerm.
    mus: bool,
    /// Panen vil også ha musebevegelse uten knapp (modus 1003).
    bevegelse: bool,
    /// Panen viser den alternative skjermen, som `less` og `vim`, og den har
    /// ingen historikk.
    alternativ: bool,
    /// Linjer i historikken til panen, og hvor mange av dem visningen er rullet
    /// opp. 0 er bunnen, slik panen står.
    historie: u32,
    rull: u32,
    /// Når panen ikke har slått på mus selv, markerer nerdr.
    markering: Option<Markering>,
    kopimodus: Option<Kopimodus>,
}

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let prompt = args.iter().position(|a| a == "--").map(|i| args.split_off(i)[1..].join(" "));
    let (Some(socket), Some(fortsett), Some(navn)) =
        (ta_verdi(&mut args, "-L"), ta_verdi(&mut args, "--fortsett"), ta_verdi(&mut args, "--navn"))
    else {
        bruk();
    };
    let json = ta_flagg(&mut args, "--json");
    let chat = ta_flagg(&mut args, "--chat");
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let socket = socket.as_deref();
    let valg = start::Valg {
        samtale: fortsett.as_deref(),
        navn: navn.as_deref().filter(|n| !n.is_empty()),
        prompt: prompt.as_deref().filter(|p| !p.is_empty()),
    };
    let flagg_for_ny = fortsett.is_some() || navn.is_some() || prompt.is_some() || chat;

    let res = match args.as_slice() {
        [] if !json && !flagg_for_ny => kjør(socket),
        ["liste"] if !flagg_for_ny => kommandoer::liste(socket, json),
        ["samtaler"] if !flagg_for_ny => kommandoer::samtaler(json),
        ["forklar"] if !json && !flagg_for_ny => kommandoer::skriv(&forklar::tekst(socket)),
        ["ny"] if chat && valg.samtale.is_none() => kommandoer::ny_chat(socket, valg, json),
        ["ny"] if valg.samtale.is_some() && !chat => kommandoer::ny(socket, None, valg, json),
        ["ny", mappe] if !chat => kommandoer::ny(socket, Some(mappe), valg, json),
        ["-h" | "--help"] => {
            print!("{}", hjelp::tekst());
            Ok(())
        }
        ["-v" | "-V" | "--version"] => {
            println!("nerdr {}", hjelp::versjon());
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
        "bruk: nerdr [liste | samtaler | ny <mappe> | ny --chat | ny --fortsett <id> [mappe] | forklar] [--json] [-L socket], eller nerdr --help\n      \
         nerdr ny tar også --navn <navn> og -- <prompt>"
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
    let mut koble = {
        let (socket, tx) = (socket.clone(), tx.clone());
        let mut nr = 0;
        move || {
            nr += 1;
            let (tx, nr) = (tx.clone(), nr);
            let kontroll = Kontroll::start(socket.as_deref(), move |m| {
                let _ = tx.send(Hendelse::Tmux(nr, m));
            })?;
            Ok((kontroll, nr))
        }
    };
    let kontroll = koble()?;
    let app_socket = socket.clone();
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
    let (kontroll, tilkobling) = kontroll;
    let ut = Tmux::ny(kontroll, Box::new(koble_igjen), app_socket, konfigkatalog());
    let mut app = App::ny(Box::new(ut), tilkobling, prosjekter_tx);
    let res = app.løkke(&mut terminal, &rx);
    let _ = execute!(io::stdout(), DisableMouseCapture, DisableBracketedPaste, DisableFocusChange);
    ratatui::restore();

    res?;
    match app.ferdig {
        Some(grunn) if !grunn.is_empty() => Err(io::Error::other(grunn)),
        _ => Ok(()),
    }
}

fn nå_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}
