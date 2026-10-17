//! `liste`, `samtaler` og `ny`: det samme som i TUI-et, for script og agenter.
//!
//! JSON-formatet har `versjon`. Felt legges til, men endres ikke, så det som
//! leser det, ikke slutter å virke.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nerdr_kjerne::paner::{FELT, Pane};
use nerdr_kjerne::register::{self, Sesjon, Tilstand};
use nerdr_kjerne::{git, samtaler, start, tmux};
use serde_json::json;

use crate::{konfigkatalog, ny};

const VERSJON: u32 = 1;

pub fn liste(socket: Option<&str>, som_json: bool) -> io::Result<()> {
    let sesjoner = register::les(&konfigkatalog().join("sessions"));
    let paner = match tmux::kjør(socket, &["list-panes", "-a", "-F", FELT]) {
        Ok(ut) => ut.lines().filter_map(Pane::fra_linje).collect(),
        Err(feil) if feil.kind() == io::ErrorKind::NotFound => return Err(feil),
        // Uten server er det ingen sesjoner.
        Err(_) => Vec::new(),
    };
    let mut claude: Vec<(Pane, &Sesjon)> = paner
        .into_iter()
        .filter_map(|p| {
            let sesjon = sesjoner.get(&p.id)?;
            Some((p, sesjon))
        })
        .collect();
    claude.sort_by_key(|(_, s)| rekkefølge(s.tilstand));

    if som_json {
        let sesjoner: Vec<_> = claude
            .iter()
            .map(|(p, s)| {
                json!({
                    "pane": p.id,
                    "sesjon": p.sesjon,
                    "vindu": p.vindu,
                    "tilstand": tilstand(s.tilstand),
                    "endret": s.endret,
                    "mappe": s.mappe,
                    "branch": s.mappe.as_deref().and_then(git::branch),
                    "oppgave": p.oppgave(),
                    "samtale": s.samtale,
                    "sett": p.sett,
                })
            })
            .collect();
        return skriv(&format!("{}\n", json!({ "versjon": VERSJON, "sesjoner": sesjoner })));
    }
    let mut ut = String::new();
    if claude.is_empty() {
        ut += "Ingen Claude-sesjoner i tmux.\n";
    }
    for (p, s) in &claude {
        let mappe = s.mappe.as_deref().map_or_else(|| format!("{}:{}", p.sesjon, p.vindu), navn);
        let branch = s.mappe.as_deref().and_then(git::branch).map(|b| format!(" › {b}")).unwrap_or_default();
        let oppgave = p.oppgave().map(|o| format!("  {o}")).unwrap_or_default();
        let _ = writeln!(ut, "{:<6}  {:<5}  {mappe}{branch}{oppgave}", tilstand(s.tilstand), p.id);
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
    let nå = SystemTime::now();
    for s in &samtaler {
        let alder = ny::alder(nå.duration_since(s.endret).unwrap_or_default());
        let _ = writeln!(ut, "{alder:>6}  {}  {}  {}", s.id, navn(&s.mappe), s.tittel);
    }
    skriv(&ut)
}

/// Uten `mappe` fortsettes samtalen i mappa den hørte til.
pub fn ny(socket: Option<&str>, mappe: Option<&str>, fortsett: Option<&str>, som_json: bool) -> io::Result<()> {
    let sesjoner = register::les(&konfigkatalog().join("sessions"));
    if let Some(id) = fortsett
        && let Some(pane) = sesjoner.iter().find(|(_, s)| s.samtale.as_deref() == Some(id)).map(|(p, _)| p)
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

fn kjører(sesjoner: &HashMap<String, Sesjon>) -> HashSet<String> {
    sesjoner.values().filter_map(|s| s.samtale.clone()).collect()
}

fn tilstand(t: Tilstand) -> &'static str {
    match t {
        Tilstand::Venter => "venter",
        Tilstand::Jobber => "jobber",
        Tilstand::Klar => "klar",
    }
}

fn rekkefølge(t: Tilstand) -> u8 {
    match t {
        Tilstand::Venter => 0,
        Tilstand::Jobber => 1,
        Tilstand::Klar => 2,
    }
}

fn navn(mappe: &Path) -> String {
    mappe.file_name().map_or_else(|| mappe.display().to_string(), |n| n.to_string_lossy().into())
}

fn ms(tid: SystemTime) -> u64 {
    tid.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}
