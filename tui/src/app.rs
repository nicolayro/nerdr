//! Tilstanden til nerdr og hendelsesløkka.

use nerdr_kjerne::{git, konfigkatalog};
use nerdr_kjerne::kontroll::{Melding, dobbelt_sitat, sitat};
use nerdr_kjerne::paner::Pane;
use nerdr_kjerne::sgr::Linje;
use ratatui::DefaultTerminal;
use ratatui::crossterm::execute;
use ratatui::crossterm::event::{Event, KeyEventKind};
use ratatui::crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use ratatui::layout::{Rect, Size};
use std::io;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};
use crate::{AktivPane, Fokus, Hendelse, Svar, bindinger, bokser, taster, tegn, utseende};
use crate::agent::Agent;
use crate::grupper::Plass;
use crate::omverden::Omverden;
use crate::popup::Popup;
use crate::tegn::Mål;
use crate::utseende::Utseende;

const LISTE_INTERVALL: Duration = Duration::from_secs(2);
/// Øvre grense for hvor ofte panen hentes på nytt, omtrent 60 bilder i sekundet.
const HENTE_INTERVALL: Duration = Duration::from_millis(16);
/// Så lenge venter vi på at en synkronisert oppdatering blir ferdig, i tilfelle
/// avslutningen aldri kommer.
const SYNK_MAKS: Duration = Duration::from_millis(200);
const KOPIERT_VISES: Duration = Duration::from_millis(1500);
const BESKJED_VISES: Duration = Duration::from_secs(4);
/// Uten svar fra tmux så lenge står klienten fast, og panene blir tomme til
/// nerdr kobler til på nytt. Det skal ikke skje etter `kontroll::for_seg`.
const SVAR_MAKS: Duration = Duration::from_secs(5);
/// Global option i tmux med bredden på lista, og `stripe` etter når den er en
/// stripe, så den er den samme neste gang nerdr starter.
const LISTE_OPTION: &str = "@nerdr-liste";

mod svar;
mod vindu;
mod tast;
mod mus;
mod liste;
mod handling;

pub(crate) struct App {
    pub(crate) ut: Box<dyn Omverden>,
    pub(crate) koblet_igjen: Option<Instant>,
    /// Nummeret til tilkoblingen. Meldinger fra en nerdr har gitt opp, hoppes over.
    pub(crate) tilkobling: u64,
    /// Siden da nerdr sist fikk et svar, så lenge noen mangler.
    pub(crate) venter_siden: Option<Instant>,
    /// For det som leses i egne tråder.
    pub(crate) tx: Sender<Hendelse>,
    pub(crate) kø: VecDeque<Svar>,

    /// Panene som har en agent, i lista.
    pub(crate) claude_paner: Vec<Pane>,
    /// Pane-id → det kortet i lista står for.
    pub(crate) agenter: HashMap<String, Agent>,
    /// Sesjonene nerdr har laget, og siden når de har vært uten Claude.
    pub(crate) uten_claude: HashMap<String, Instant>,
    /// Alle panene på serveren.
    pub(crate) alle: Vec<Pane>,
    pub(crate) lister: bool,
    pub(crate) list_igjen: bool,
    /// Panen `aktiver` gjorde aktiv mens en liste var på vei. Den lista er
    /// eldre enn `select-pane`, og ville gitt `x` rett etter `o` den gamle.
    pub(crate) aktivert: Option<String>,
    pub(crate) sist_listet: Option<Instant>,
    /// Konfigurasjonskatalogen til Claude Code, med registeret i `sessions`
    /// og samtalene i `projects`.
    pub(crate) konfig: PathBuf,
    /// Lest bare når lista er tom, for det er da du lurer på hvor de er.
    pub(crate) utenfor_tmux: usize,
    /// Pane-id → repoet og branchen mappa til Claude-en står i.
    pub(crate) repoer: HashMap<String, git::Repo>,
    /// Pane-id → da du sist var inne i panen, også skrevet til panen i tmux.
    /// En Claude nerdr ikke har sett før, regnes som sett, så ingenting havner
    /// i «venter på deg» første gang nerdr starter.
    pub(crate) sett: HashMap<String, u64>,
    /// Panen du er inne i, og plassen den hadde da du gikk inn. Den blir stående
    /// der i delen til du går ut, så kortet ikke flytter seg for hver melding.
    pub(crate) låst: Option<(String, Plass)>,
    /// Panen du sist gikk ut av, med plassen fra `låst`. Den holdes til
    /// registeret er lest på nytt, fordi tilstanden nerdr har, kan være eldre
    /// enn prompten du nettopp sendte.
    pub(crate) forlatt: Option<(String, Plass)>,
    /// Prosjektene som er brettet sammen, når lista er ordnet etter mappe.
    pub(crate) brettet: HashSet<String>,

    /// Claude-en som er valgt i lista, og som vises til høyre.
    pub(crate) valgt: Option<Pane>,
    /// Pane-id-en til kortet piltastene står på i lista. Den byttes til først
    /// med `⏎`, så du kan bla forbi andre uten å bytte sesjon i tmux.
    pub(crate) peker: Option<String>,
    /// Bredden på lista med skillelinja, når den ikke er en stripe.
    pub(crate) liste_bredde: u16,
    /// Lista er en stripe med bare tegnene.
    pub(crate) liste_stripe: bool,
    pub(crate) drar_skille: bool,
    /// Første rad i lista som vises, når den ikke får plass.
    pub(crate) liste_rull: u16,
    /// Lista ruller så den valgte vises, til du ruller den selv.
    pub(crate) liste_følger: bool,
    /// Vinduet som vises til høyre: vinduet til den valgte, eller et annet
    /// vindu i samme sesjon.
    pub(crate) vist: Option<String>,
    pub(crate) forrige_vindu: Option<String>,
    /// Panene i vinduet, der tmux har plassert dem.
    pub(crate) vindu: Vec<Pane>,
    /// Pane-id → innholdet.
    pub(crate) bilder: HashMap<String, Vec<Linje>>,
    /// Tilstanden til den aktive panen i vinduet, den tastene går til.
    pub(crate) aktiv_pane: AktivPane,
    pub(crate) utdatert: bool,
    pub(crate) henter: bool,
    pub(crate) sist_hentet: Instant,
    /// Pane-id → siden når panen har vært midt i en synkronisert oppdatering.
    pub(crate) synk: HashMap<String, Instant>,
    /// Pane-id → slutten av forrige `%output`, i tilfelle en synkmarkør er
    /// delt over to.
    pub(crate) output_hale: HashMap<String, String>,
    /// Visningen og rutene boksene trengte, da klientstørrelsen sist ble sendt.
    pub(crate) størrelser: HashMap<String, (u16, u16)>,

    pub(crate) fokus: Fokus,
    pub(crate) prefiks: taster::Prefiks,
    pub(crate) utseende: Utseende,
    /// Hjelp, innstillinger, spørsmål, handlinger eller mappevelger.
    pub(crate) popup: Option<Popup>,
    /// Prefikset er trykket, og neste tast er en kommando.
    pub(crate) etter_prefiks: bool,
    pub(crate) bindinger: bindinger::Bindinger,
    /// Tastene uten prefiks fra `bind -n`, som gjelder i panen.
    pub(crate) root: bindinger::Bindinger,
    /// Til da går en tast med `-r` til tmux-kommandoen uten prefikset.
    pub(crate) gjentar: Option<Instant>,
    pub(crate) visning: Rect,
    pub(crate) skjerm: Rect,
    /// Kolonne og rad, til terminalen mister fokus. Terminalen sier ikke fra
    /// når musen forlater vinduet.
    pub(crate) musen: Option<(u16, u16)>,
    /// Det som sist ble klikket på, og når, så et klikk til er et dobbeltklikk.
    pub(crate) sist_klikket: Option<(Mål, Instant)>,
    /// Det forrige klikket gikk ikke til panen, og det gjør ikke slippet heller.
    pub(crate) svelg_slipp: bool,
    /// Da markeringen sist ble kopiert, så det kan stå en liten stund.
    pub(crate) kopiert: Option<Instant>,
    /// Det nerdr ikke kunne gjøre, og når det skjedde.
    pub(crate) beskjed: Option<(String, Instant)>,
    /// Panene tmux har fått fargene til terminalen for. En control mode-klient
    /// har ingen terminal tmux kan spørre, så uten dem svarer tmux svart når en
    /// pane spør om bakgrunnen, og Claude Code med temaet «auto» blir mørk også
    /// i en lys terminal.
    pub(crate) fargelagt: HashSet<String>,
    /// Panen nerdr skal gå inn i når den dukker opp i `list-panes`.
    pub(crate) gå_inn_i: Option<String>,
    pub(crate) ferdig: Option<String>,
}

impl App {
    pub(crate) fn ny(ut: Box<dyn Omverden>, tilkobling: u64, tx: Sender<Hendelse>) -> App {
        App {
            ut,
            tx,
            koblet_igjen: None,
            tilkobling,
            venter_siden: None,
            kø: VecDeque::new(),
            claude_paner: Vec::new(),
            agenter: HashMap::new(),
            uten_claude: HashMap::new(),
            alle: Vec::new(),
            lister: false,
            list_igjen: false,
            aktivert: None,
            sist_listet: None,
            konfig: konfigkatalog(),
            utenfor_tmux: 0,
            repoer: HashMap::new(),
            sett: HashMap::new(),
            låst: None,
            forlatt: None,
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
            aktiv_pane: AktivPane::default(),
            utdatert: false,
            henter: false,
            sist_hentet: Instant::now(),
            synk: HashMap::new(),
            output_hale: HashMap::new(),
            størrelser: HashMap::new(),
            fokus: Fokus::Liste,
            prefiks: taster::Prefiks::fra_miljø().unwrap_or_default(),
            utseende: Utseende::ny(bokser::Rammer::Linjer),
            popup: None,
            etter_prefiks: false,
            bindinger: bindinger::Bindinger::default(),
            root: bindinger::Bindinger::default(),
            gjentar: None,
            visning: Rect::default(),
            skjerm: Rect::default(),
            musen: None,
            sist_klikket: None,
            svelg_slipp: false,
            kopiert: None,
            beskjed: None,
            fargelagt: HashSet::new(),
            gå_inn_i: None,
            ferdig: None,
        }
    }

    pub(crate) fn løkke(&mut self, terminal: &mut DefaultTerminal, rx: &Receiver<Hendelse>) -> io::Result<()> {
        if taster::Prefiks::fra_miljø().is_none() {
            self.send("show-options -gv prefix", Svar::Prefiks);
        }
        self.send("list-keys -T prefix", Svar::Bindinger);
        self.send("list-keys -T root", Svar::Root);
        self.send("show-options -gv repeat-time", Svar::Gjentatid);
        self.send(&format!("show-options -gqv {LISTE_OPTION}"), Svar::Liste);
        self.send(&format!("show-options -gqv {}", utseende::OPTION), Svar::Utseende);
        self.list_paner();
        let mut tegne = true;
        while self.ferdig.is_none() {
            if tegne {
                self.forbered_tegning(terminal.size()?);
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
            tegne |= self.tikk();
        }
        Ok(())
    }

    /// Regner ut det tegningen og museklikkene bygger på, fra størrelsen på
    /// terminalen.
    pub(crate) fn forbered_tegning(&mut self, størrelse: Size) {
        self.utseende.bredde = utseende::Bredde::nærmest(self.liste_bredde);
        let liste = if self.liste_stripe { tegn::stripebredde(self.utseende.paner) } else { self.liste_bredde };
        self.skjerm = Rect::new(0, 0, størrelse.width, størrelse.height);
        self.visning = tegn::visning(størrelse, liste, self.utseende.paner);
        let rader = tegn::menyrader(self);
        if let Some(popup) = &mut self.popup {
            let antall = popup.antall_treff();
            if let Some(felt) = popup.felt() {
                felt.rull = tegn::følg(felt.valgt, felt.rull, rader, antall);
            }
        }
        let (utsnitt, valgt) = tegn::utsnitt(self);
        self.liste_rull = match valgt.filter(|_| self.liste_følger) {
            Some(valgt) => utsnitt.følg(valgt).rull,
            None => utsnitt.rull,
        };
    }

    /// Det som skjer i hver runde etter at hendelsene er håndtert, og som
    /// bare avhenger av tiden. Sier om det må tegnes på nytt.
    pub(crate) fn tikk(&mut self) -> bool {
        let mut tegne = false;
        if self.kopiert.is_some_and(|t| t.elapsed() >= KOPIERT_VISES) {
            self.kopiert = None;
            tegne = true;
        }
        if self.beskjed.as_ref().is_some_and(|(_, t)| t.elapsed() >= BESKJED_VISES) {
            self.beskjed = None;
            tegne = true;
        }
        if self.venter_siden.is_some_and(|t| t.elapsed() >= SVAR_MAKS) {
            // Går det ikke, prøves det igjen like lenge etter.
            self.venter_siden = Some(Instant::now());
            tegne |= self.koble_til_igjen();
        }
        if self.sist_listet.is_some_and(|t| t.elapsed() >= LISTE_INTERVALL) {
            self.list_paner();
        }
        if self.tid_til_henting() == Some(Duration::ZERO) {
            self.hent();
        }
        tegne
    }

    /// Hvor lenge det er til panen kan hentes på nytt, eller `None` når det
    /// ikke er noe nytt å hente eller en henting allerede pågår.
    pub(crate) fn tid_til_henting(&self) -> Option<Duration> {
        if !self.utdatert || self.henter {
            return None;
        }
        let intervall = HENTE_INTERVALL.saturating_sub(self.sist_hentet.elapsed());
        let synk = self.synk.values().min().map_or(Duration::ZERO, |siden| SYNK_MAKS.saturating_sub(siden.elapsed()));
        Some(intervall.max(synk))
    }

    /// Svarene på det som var sendt, kommer aldri, og den nye klienten står i
    /// sesjonen tmux velger. Kobles det fra igjen med en gang, gir nerdr opp.
    pub(crate) fn koble_til_igjen(&mut self) -> bool {
        if self.koblet_igjen.is_some_and(|t| t.elapsed() < Duration::from_secs(1)) {
            return false;
        }
        let Ok(nr) = self.ut.koble_igjen() else { return false };
        self.tilkobling = nr;
        self.koblet_igjen = Some(Instant::now());
        self.kø.clear();
        self.venter_siden = None;
        (self.lister, self.list_igjen, self.henter) = (false, false, false);
        self.aktivert = None;
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
    pub(crate) fn gi_farger(&mut self, pane: &str) {
        let Some((fg, bg)) = tegn::farger() else { return };
        for (nr, (r, g, b)) in [(10, fg), (11, bg)] {
            let svar = format!("{pane}:\x1b]{nr};rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}\x07");
            self.send(&format!("refresh-client -r {}", dobbelt_sitat(&svar)), Svar::Ingenting);
        }
        self.fargelagt.insert(pane.to_string());
    }

    pub(crate) fn send(&mut self, kommando: &str, svar: Svar) {
        match self.ut.send(kommando) {
            Ok(()) => {
                self.kø.push_back(svar);
                self.venter_siden.get_or_insert_with(Instant::now);
            }
            Err(feil) => self.ferdig = Some(format!("tmux: {feil}")),
        }
    }

    pub(crate) fn håndter(&mut self, h: Hendelse) -> bool {
        match h {
            Hendelse::Tmux(nr, _) if nr != self.tilkobling => false,
            Hendelse::Tmux(_, Melding::Svar { ok, linjer }) => {
                let svar = self.kø.pop_front();
                self.venter_siden = (!self.kø.is_empty()).then(Instant::now);
                svar.is_some_and(|svar| self.håndter_svar(svar, ok, linjer))
            }
            Hendelse::Tmux(_, Melding::Output { pane, data }) => {
                if self.vindu.iter().any(|p| p.id == pane) {
                    self.output(pane, &data);
                }
                false
            }
            Hendelse::Tmux(_, Melding::Endret) => {
                self.list_paner();
                false
            }
            Hendelse::Tmux(_, Melding::Slutt(grunn)) => {
                if grunn.is_empty() && self.koble_til_igjen() {
                    return true;
                }
                self.ferdig = Some(if grunn.is_empty() { "tmux koblet fra".into() } else { grunn });
                false
            }
            Hendelse::Term(Event::Key(k)) if k.kind == KeyEventKind::Press => self.tast(k),
            Hendelse::Term(Event::Mouse(m)) => self.mus(m),
            Hendelse::Term(Event::Paste(tekst)) => match &mut self.popup {
                Some(popup) => popup.lim_inn(&tekst),
                None => {
                    self.lim_inn(&tekst);
                    false
                }
            },
            Hendelse::Mapper(fra, mapper, ferdig) => match &mut self.popup {
                Some(Popup::Velger(velger)) if velger.leser(&fra) => {
                    velger.legg_til(mapper, ferdig);
                    true
                }
                _ => false,
            },
            Hendelse::Term(Event::FocusLost) => {
                self.send("refresh-client -f ignore-size", Svar::Ingenting);
                self.musen.take().is_some()
            }
            Hendelse::Term(Event::FocusGained) => {
                self.ta_plassen();
                false
            }
            Hendelse::Term(Event::Resize(..)) => true,
            Hendelse::Term(_) => false,
        }
    }
}
