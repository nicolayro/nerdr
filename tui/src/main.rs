mod taster;
mod tegn;

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};
use std::{env, io, thread};

use nerdr_kjerne::git::Git;
use nerdr_kjerne::kontroll::{Kontroll, Melding, sitat, synkronisert_etter};
use nerdr_kjerne::oversikt::{self, FELT, Gruppe, Pane, Rad};
use nerdr_kjerne::sgr::{self, Linje};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::Rect;

const SAMLE_INTERVALL: Duration = Duration::from_secs(2);
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
    Skjerm(String),
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
    git: Git,
    /// Panen nerdr selv kjører i, når den startes inne i tmux. Den holdes
    /// utenfor, ellers viser den seg selv i seg selv.
    eget_pane: Option<String>,

    paner: Vec<Pane>,
    skjermer: HashMap<String, String>,
    gjenstår: usize,
    samler: bool,
    sist_samlet: Option<Instant>,
    grupper: Vec<Gruppe>,

    valgt: Option<String>,
    vist: Option<String>,
    vist_sesjon: Option<String>,
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
    linjekart: Vec<Option<String>>,
    ferdig: Option<String>,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (socket, rest) = match args.as_slice() {
        [flagg, socket, rest @ ..] if flagg == "-L" => (Some(socket.as_str()), rest),
        rest => (None, rest),
    };
    let res = match rest {
        [] => kjør(socket),
        [json_flagg] if json_flagg == "--json" => json(socket),
        _ => {
            eprintln!("bruk: nerdr [-L socket] [--json]");
            std::process::exit(2);
        }
    };
    if let Err(feil) = res {
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
    let mut app = App::ny(kontroll, eget_pane(socket));
    let res = app.løkke(&mut terminal, &rx);
    let _ = execute!(io::stdout(), DisableMouseCapture, DisableBracketedPaste);
    ratatui::restore();

    res?;
    match app.ferdig {
        Some(grunn) if !grunn.is_empty() => Err(io::Error::other(grunn)),
        _ => Ok(()),
    }
}

/// Pane-id-er er bare unike innenfor én server, så `$TMUX_PANE` gjelder bare
/// når `$TMUX` peker på den samme serveren som nerdr viser.
fn eget_pane(socket: Option<&str>) -> Option<String> {
    let tmux = env::var("TMUX").ok()?;
    let sti = Path::new(tmux.split(',').next()?);
    let samme = sti.file_name()?.to_str()? == socket.unwrap_or("default");
    samme.then(|| env::var("TMUX_PANE").ok()).flatten()
}

impl App {
    fn ny(kontroll: Kontroll, eget_pane: Option<String>) -> App {
        App {
            kontroll,
            kø: VecDeque::new(),
            git: Git::default(),
            eget_pane,
            paner: Vec::new(),
            skjermer: HashMap::new(),
            gjenstår: 0,
            samler: false,
            sist_samlet: None,
            grupper: Vec::new(),
            valgt: None,
            vist: None,
            vist_sesjon: None,
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
            linjekart: Vec::new(),
            ferdig: None,
        }
    }

    fn løkke(&mut self, terminal: &mut DefaultTerminal, rx: &Receiver<Hendelse>) -> io::Result<()> {
        self.samle();
        let mut tegne = true;
        while self.ferdig.is_none() {
            if tegne {
                self.visning = tegn::visning(terminal.size()?);
                let sidelinjer = tegn::sidebar(self);
                self.linjekart = sidelinjer.iter().map(|l| l.mål.clone()).collect();
                terminal.draw(|f| tegn::alt(f, self, &sidelinjer))?;
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

            if self.sist_samlet.is_some_and(|t| t.elapsed() >= SAMLE_INTERVALL) {
                self.samle();
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

    fn rader(&self) -> impl Iterator<Item = &Rad> {
        self.grupper.iter().flat_map(|g| &g.rader)
    }

    fn håndter(&mut self, h: Hendelse) -> bool {
        match h {
            Hendelse::Tmux(Melding::Svar { ok, linjer }) => match self.kø.pop_front() {
                Some(venter) => self.svar(venter, ok, linjer),
                None => false,
            },
            Hendelse::Tmux(Melding::Output { pane, data }) => {
                if self.vist.as_ref() == Some(&pane) {
                    self.utdatert = true;
                    let søk = std::mem::take(&mut self.output_hale) + &data;
                    if let Some(synk) = synkronisert_etter(&søk) {
                        if synk && !self.synk {
                            self.synk_siden = Instant::now();
                        }
                        self.synk = synk;
                    }
                    // Markøren er 11 tegn, så de siste 10 holder for å fange
                    // en som er delt mellom to biter.
                    let fra = søk.char_indices().rev().nth(9).map_or(0, |(i, _)| i);
                    self.output_hale = søk[fra..].to_string();
                }
                false
            }
            Hendelse::Tmux(Melding::Endret) => {
                self.samle();
                false
            }
            Hendelse::Tmux(Melding::Slutt(grunn)) => {
                self.ferdig = Some(if grunn.is_empty() { "tmux koblet fra".into() } else { grunn });
                false
            }
            Hendelse::Term(Event::Key(k)) if k.kind == KeyEventKind::Press => self.tast(k),
            Hendelse::Term(Event::Mouse(m)) => self.mus(m),
            Hendelse::Term(Event::Paste(tekst)) => {
                if let (Fokus::Pane, Some(id)) = (self.fokus, self.vist.clone()) {
                    for del in taster::lim_inn(&tekst) {
                        self.send(&format!("send-keys -t {} {del}", sitat(&id)), Venter::Ingenting);
                    }
                }
                false
            }
            Hendelse::Term(Event::Resize(..)) => true,
            Hendelse::Term(_) => false,
        }
    }

    fn svar(&mut self, venter: Venter, ok: bool, linjer: Vec<String>) -> bool {
        match venter {
            Venter::Paner => {
                let eget = self.eget_pane.clone();
                self.paner = linjer
                    .iter()
                    .filter_map(|l| Pane::fra_linje(l))
                    .filter(|p| ok && Some(&p.id) != eget.as_ref())
                    .collect();
                self.skjermer.clear();
                let claude: Vec<String> = self
                    .paner
                    .iter()
                    .filter(|p| p.er_claude() && !p.er_sidebar)
                    .map(|p| p.id.clone())
                    .collect();
                self.gjenstår = claude.len();
                for id in claude {
                    self.send(&format!("capture-pane -p -t {} -S -40", sitat(&id)), Venter::Skjerm(id));
                }
                self.gjenstår == 0 && self.samlet()
            }
            Venter::Skjerm(id) => {
                if ok {
                    self.skjermer.insert(id, linjer.join("\n"));
                }
                self.gjenstår = self.gjenstår.saturating_sub(1);
                self.gjenstår == 0 && self.samlet()
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

    fn samle(&mut self) {
        if !self.samler {
            self.samler = true;
            self.send(&format!("list-panes -a -F {}", sitat(FELT)), Venter::Paner);
        }
    }

    fn samlet(&mut self) -> bool {
        self.grupper = oversikt::bygg(&self.paner, &self.skjermer, &mut self.git);
        self.samler = false;
        self.sist_samlet = Some(Instant::now());

        let finnes = |app: &App, id: &Option<String>| {
            id.as_ref().is_some_and(|id| app.rader().any(|r| &r.pane.id == id))
        };
        if !finnes(self, &self.valgt) {
            let viktigst = self.rader().max_by_key(|r| r.tilstand.rang()).map(|r| r.pane.id.clone());
            self.valgt = viktigst;
        }
        if !finnes(self, &self.vist) {
            self.vist = None;
            if let Some(id) = self.valgt.clone() {
                self.vis(&id);
            }
        }
        true
    }

    fn vis(&mut self, id: &str) {
        let Some(pane) = self.paner.iter().find(|p| p.id == id).cloned() else { return };
        if self.vist_sesjon.as_ref() != Some(&pane.sesjon_id) {
            self.send(&format!("switch-client -t {}", sitat(&pane.sesjon_id)), Venter::Ingenting);
            self.vist_sesjon = Some(pane.sesjon_id.clone());
        }
        // Vinduet må være sesjonens aktive for å få størrelsen fra denne
        // klienten. Det bytter også vindu for andre klienter i samme sesjon.
        self.send(&format!("select-window -t {}", sitat(&pane.vindu_id)), Venter::Ingenting);
        self.vist = Some(pane.id);
        self.bilde.clear();
        self.markør = None;
        self.størrelse = None;
        self.utdatert = true;
        self.synk = false;
        self.output_hale.clear();
    }

    fn hent(&mut self) {
        let Some(id) = self.vist.as_deref().map(sitat) else { return };
        self.send(&format!("capture-pane -p -e -N -t {id}"), Venter::Bilde);
        let felt = sitat("#{cursor_x}\t#{cursor_y}\t#{cursor_flag}");
        self.send(&format!("display-message -p -t {id} {felt}"), Venter::Markør);
        self.utdatert = false;
        self.henter = true;
        self.sist_hentet = Instant::now();
    }

    /// Klienten får størrelsen som gjør at den viste panen fyller visningen.
    /// Resten av vinduet, som sidebar-kolonnene fra tmux-oppsettet, legges til.
    fn synk_størrelse(&mut self) {
        let Some(pane) = self.vist.as_ref().and_then(|id| self.paner.iter().find(|p| &p.id == id)) else {
            return;
        };
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
            if let (Some(id), Some(args)) = (self.vist.clone(), taster::send_keys(&k)) {
                self.send(&format!("send-keys -t {} {args}", sitat(&id)), Venter::Ingenting);
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
            KeyCode::Enter | KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                self.fokus = Fokus::Pane
            }
            _ => return false,
        }
        true
    }

    fn flytt(&mut self, steg: isize) {
        let ider: Vec<String> = self.rader().map(|r| r.pane.id.clone()).collect();
        if ider.is_empty() {
            return;
        }
        let nå = self.valgt.as_ref().and_then(|v| ider.iter().position(|i| i == v)).unwrap_or(0);
        let ny = (nå as isize + steg).clamp(0, ider.len() as isize - 1) as usize;
        self.valgt = Some(ider[ny].clone());
        if self.vist.as_ref() != Some(&ider[ny]) {
            self.vis(&ider[ny]);
        }
    }

    fn mus(&mut self, m: MouseEvent) -> bool {
        if m.kind != MouseEventKind::Down(MouseButton::Left) {
            return false;
        }
        if m.column < self.visning.x {
            let Some(Some(id)) = self.linjekart.get(m.row as usize).cloned() else { return false };
            self.valgt = Some(id.clone());
            if self.vist.as_ref() != Some(&id) {
                self.vis(&id);
            }
        }
        self.fokus = Fokus::Pane;
        true
    }
}

/// Oversikten som JSON, i formatet `sesjoner.py --json` har, uten `aktiv`.
fn json(socket: Option<&str>) -> io::Result<()> {
    let (tx, rx) = mpsc::channel();
    let mut kontroll = Kontroll::start(socket, move |m| {
        let _ = tx.send(m);
    })?;
    let mut spør = |kommando: &str| -> io::Result<Option<Vec<String>>> {
        kontroll.send(kommando)?;
        loop {
            match rx.recv() {
                Ok(Melding::Svar { ok, linjer }) => return Ok(ok.then_some(linjer)),
                Ok(Melding::Slutt(grunn)) => return Err(io::Error::other(grunn)),
                Ok(_) => {}
                Err(_) => return Err(io::Error::other("tmux koblet fra")),
            }
        }
    };

    let Ok(linjer) = spør(&format!("list-panes -a -F {}", sitat(FELT))) else {
        // Ingen server eller ingen sesjoner gir tom oversikt, ikke feil.
        println!("{{\"telling\": {{}}, \"sesjoner\": []}}");
        return Ok(());
    };
    let paner: Vec<Pane> = linjer.unwrap_or_default().iter().filter_map(|l| Pane::fra_linje(l)).collect();
    let mut skjermer = HashMap::new();
    for p in paner.iter().filter(|p| p.er_claude() && !p.er_sidebar) {
        if let Some(linjer) = spør(&format!("capture-pane -p -t {} -S -40", sitat(&p.id)))? {
            skjermer.insert(p.id.clone(), linjer.join("\n"));
        }
    }
    let grupper = oversikt::bygg(&paner, &skjermer, &mut Git::default());
    println!("{}", som_json(&grupper));
    Ok(())
}

fn som_json(grupper: &[Gruppe]) -> String {
    let mut telling: Vec<(&str, usize)> = Vec::new();
    let mut sesjoner = Vec::new();
    for gruppe in grupper {
        for r in &gruppe.rader {
            match telling.iter_mut().find(|(t, _)| *t == r.tilstand.navn()) {
                Some((_, n)) => *n += 1,
                None => telling.push((r.tilstand.navn(), 1)),
            }
            let target = if r.etikett.contains(':') {
                format!("{}:{}", r.pane.sesjon, r.pane.vindu)
            } else {
                r.pane.sesjon.clone()
            };
            let felt = [
                ("target", target.as_str()),
                ("sesjon", &r.pane.sesjon),
                ("vindu", &r.pane.vindu),
                ("repo", &gruppe.repo),
                ("branch", &r.branch),
                ("tilstand", r.tilstand.navn()),
                ("status", &r.status),
                ("oppgave", &r.oppgave),
                ("spørsmål", &r.spørsmål),
                ("kontekst", &r.kontekst),
            ];
            let felt: Vec<String> =
                felt.iter().map(|(k, v)| format!("      {}: {}", streng(k), streng(v))).collect();
            sesjoner.push(format!("    {{\n{}\n    }}", felt.join(",\n")));
        }
    }
    let telling: Vec<String> = telling.iter().map(|(t, n)| format!("    {}: {n}", streng(t))).collect();
    format!(
        "{{\n  \"telling\": {{\n{}\n  }},\n  \"sesjoner\": [\n{}\n  ]\n}}",
        telling.join(",\n"),
        sesjoner.join(",\n")
    )
}

fn streng(s: &str) -> String {
    let mut ut = String::from('"');
    for c in s.chars() {
        match c {
            '"' => ut.push_str("\\\""),
            '\\' => ut.push_str("\\\\"),
            '\n' => ut.push_str("\\n"),
            c if (c as u32) < 0x20 => ut.push_str(&format!("\\u{:04x}", c as u32)),
            c => ut.push(c),
        }
    }
    ut.push('"');
    ut
}
