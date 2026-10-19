//! `liste`, `samtaler` og `ny`: det samme som i TUI-et, for script og agenter.
//!
//! JSON-formatet har `versjon`. Felt legges til, men endres ikke, så det som
//! leser det, ikke slutter å virke.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use nerdr_kjerne::merke::Merke;
use nerdr_kjerne::paner::{FELT, Pane};
use nerdr_kjerne::register::{self, Claude, Tilstand};
use nerdr_kjerne::{git, konfigkatalog, samtaler, start, tmux};
use serde_json::json;

use crate::grupper::{self, Plass, Rad, glyf, tittel};
use crate::ny;
use crate::tekst::{mappenavn, sted, varighet};

const VERSJON: u32 = 1;

/// Gruppert og sortert som lista i TUI-et.
pub fn liste(socket: Option<&str>, som_json: bool) -> io::Result<()> {
    let claude = register::les(&konfigkatalog().join("sessions"));
    let paner: Vec<Pane> = match tmux::kjør(socket, &["list-panes", "-a", "-F", FELT]) {
        Ok(ut) => ut.lines().filter_map(Pane::fra_linje).filter(|p| claude.contains_key(&p.id)).collect(),
        Err(feil) if feil.kind() == io::ErrorKind::NotFound => return Err(feil),
        // Uten server er det ingen sesjoner.
        Err(_) => Vec::new(),
    };
    // En Claude ingen nerdr har vært inne i, regnes som sett, som i TUI-et.
    let merke = |p: &Pane| Some(Merke::fra(claude.get(&p.id)?, p.sett.unwrap_or(u64::MAX)));
    let rader = grupper::rader(&paner, |p| Some(Plass::fra(merke(p)?, claude.get(&p.id).map(|c| c.endret))));

    if som_json {
        let sesjoner: Vec<_> = rader
            .iter()
            .filter_map(|r| match r {
                Rad::Pane(p, m) => Some((p, m, claude.get(&p.id)?)),
                _ => None,
            })
            .map(|(p, m, c)| {
                let repo = c.mappe.as_deref().and_then(git::les);
                json!({
                    "pane": p.id,
                    "sesjon": p.sesjon,
                    "vindu": p.vindu,
                    "tilstand": c.tilstand.map(tilstand),
                    "merke": m.navn(),
                    "gruppe": m.gruppe().navn(),
                    "endret": c.endret,
                    "mappe": c.mappe,
                    "repo": repo.as_ref().map(|r| &r.navn),
                    "branch": repo.and_then(|r| r.branch),
                    "oppgave": p.oppgave(),
                    "samtale": c.samtale,
                    "sett": p.sett,
                })
            })
            .collect();
        return skriv(&format!("{}\n", json!({ "versjon": VERSJON, "sesjoner": sesjoner })));
    }
    let mut ut = String::new();
    if rader.is_empty() {
        ut += "Ingen Claude-sesjoner i tmux.\n";
    }
    for rad in &rader {
        match rad {
            Rad::Tom => ut.push('\n'),
            Rad::Strek => {}
            Rad::Overskrift(gruppe) => ut += &format!("{}\n", tittel(*gruppe)),
            Rad::Pane(p, m) => {
                let mappe = claude.get(&p.id).and_then(|c| c.mappe.as_deref());
                let repo = mappe.and_then(git::les);
                let navn = sted(p, mappe, repo.as_ref());
                let branch = repo.and_then(|r| r.branch).map(|b| format!(" › {b}")).unwrap_or_default();
                let oppgave = p.oppgave().map(|o| format!("  {o}")).unwrap_or_default();
                let _ = writeln!(ut, "{}  {:<5}  {navn}{branch}{oppgave}", glyf(*m), p.id);
            }
        }
    }
    skriv(&ut)
}

pub fn samtaler(som_json: bool) -> io::Result<()> {
    let kjører = kjører(&register::les(&konfigkatalog().join("sessions")));
    let samtaler = samtaler::siste(&konfigkatalog().join("projects"), ny::SAMTALER, &kjører);

    if som_json {
        let samtaler: Vec<_> = samtaler
            .iter()
            .map(|s| json!({ "id": s.id, "mappe": s.mappe, "tittel": s.tittel, "endret": ms(s.endret) }))
            .collect();
        return skriv(&format!("{}\n", json!({ "versjon": VERSJON, "samtaler": samtaler })));
    }
    let mut ut = String::new();
    if samtaler.is_empty() {
        ut += "Ingen tidligere samtaler.\n";
    }
    for s in &samtaler {
        let alder = varighet(crate::nå_ms().saturating_sub(ms(s.endret)));
        let _ = writeln!(ut, "{alder:>4}  {}  {}  {}", s.id, mappenavn(&s.mappe), s.tittel);
    }
    skriv(&ut)
}

/// Uten `mappe` fortsettes samtalen i mappa den hørte til.
pub fn ny(socket: Option<&str>, mappe: Option<&str>, fortsett: Option<&str>, som_json: bool) -> io::Result<()> {
    if let Some(id) = fortsett.filter(|id| !samtaler::gyldig_id(id)) {
        return Err(io::Error::other(format!("{id} er ikke en id for en samtale")));
    }
    let claude = register::les(&konfigkatalog().join("sessions"));
    if let Some(id) = fortsett
        && let Some(pane) = claude.iter().find(|(_, c)| c.samtale.as_deref() == Some(id)).map(|(p, _)| p)
    {
        return Err(io::Error::other(format!("samtalen {id} kjører alt, i {pane}")));
    }
    let mappe: PathBuf = match (mappe, fortsett) {
        (Some(mappe), _) => {
            fs::canonicalize(mappe).map_err(|feil| io::Error::other(format!("fant ikke mappa {mappe}: {feil}")))?
        }
        (None, Some(id)) => samtaler::finn(&konfigkatalog().join("projects"), id)
            .ok_or_else(|| io::Error::other(format!("fant ingen samtale med id {id}")))?
            .mappe,
        (None, None) => unreachable!("main krever mappe eller --fortsett"),
    };
    if !mappe.is_dir() {
        return Err(io::Error::other(format!("{} er ikke en mappe", mappe.display())));
    }

    let (sesjon, pane) = start::start(socket, &mappe, fortsett)?;
    skriv(&if som_json {
        format!("{}\n", json!({ "versjon": VERSJON, "sesjon": sesjon, "pane": pane, "mappe": mappe }))
    } else {
        format!("Startet Claude i {} ({pane}, tmux-sesjonen {sesjon}).\n", mappe.display())
    })
}

/// `println!` får panikk når den som leser, lukker, som `nerdr samtaler | head`.
fn skriv(tekst: &str) -> io::Result<()> {
    match io::stdout().lock().write_all(tekst.as_bytes()) {
        Err(feil) if feil.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        res => res,
    }
}

fn kjører(claude: &HashMap<String, Claude>) -> HashSet<String> {
    claude.values().filter_map(|c| c.samtale.clone()).collect()
}

/// Verdiene står i JSON og endres ikke, så `Spør` er fortsatt «venter» her.
fn tilstand(t: Tilstand) -> &'static str {
    match t {
        Tilstand::Spør => "venter",
        Tilstand::Jobber => "jobber",
        Tilstand::Klar => "klar",
    }
}

fn ms(tid: SystemTime) -> u64 {
    tid.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}
