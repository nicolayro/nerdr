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

use nerdr_kjerne::merke::Merke;
use nerdr_kjerne::paner::{FELT, Pane};
use nerdr_kjerne::register::{self, Claude, Tilstand};
use nerdr_kjerne::samtaler::Samtale;
use nerdr_kjerne::{git, konfigkatalog, samtaler, start, tmux};
use serde_json::{Value, json};

use crate::grupper::{self, Plassering, Rad, glyf};
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
    let rader = grupper::rader(&paner, |p| Some(Plassering::fra(merke(p)?, claude.get(&p.id).map(|c| c.endret))));

    if som_json {
        return skriv(&format!("{}\n", liste_json(&rader, &claude)));
    }
    let mut ut = String::new();
    if rader.is_empty() {
        ut += "Ingen Claude-sesjoner i tmux.\n";
    }
    for rad in &rader {
        match rad {
            Rad::Tom | Rad::Brettet(..) => ut.push('\n'),
            Rad::Overskrift(o) => ut += &format!("{}\n", o.tittel()),
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
        return skriv(&format!("{}\n", samtaler_json(&samtaler)));
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
pub fn ny(socket: Option<&str>, mappe: Option<&str>, valg: start::Valg, som_json: bool) -> io::Result<()> {
    let fortsett = valg.samtale;
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

    let (sesjon, pane) = start::start(socket, &mappe, valg)?;
    skriv(&if som_json {
        format!("{}\n", ny_json(&sesjon, &pane, &mappe, valg.navn))
    } else {
        format!("Startet Claude i {} ({pane}, tmux-sesjonen {sesjon}).\n", mappe.display())
    })
}

pub fn ny_chat(socket: Option<&str>, valg: start::Valg, som_json: bool) -> io::Result<()> {
    let mappe = start::chatmappe().map_err(|feil| io::Error::other(format!("fikk ikke laget mappa for chat: {feil}")))?;
    ny(socket, Some(&mappe.to_string_lossy()), valg, som_json)
}

/// `println!` får panikk når den som leser, lukker, som `nerdr samtaler | head`.
pub fn skriv(tekst: &str) -> io::Result<()> {
    match io::stdout().lock().write_all(tekst.as_bytes()) {
        Err(feil) if feil.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        res => res,
    }
}

/// Det `liste --json` skriver. Feltnavnene er kontrakten med script og agenter,
/// så de endres ikke, og tester låser dem.
fn liste_json(rader: &[Rad], claude: &HashMap<String, Claude>) -> Value {
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
    json!({ "versjon": VERSJON, "sesjoner": sesjoner })
}

fn samtaler_json(samtaler: &[Samtale]) -> Value {
    let samtaler: Vec<_> = samtaler
        .iter()
        .map(|s| json!({ "id": s.id, "mappe": s.mappe, "tittel": s.tittel, "endret": ms(s.endret) }))
        .collect();
    json!({ "versjon": VERSJON, "samtaler": samtaler })
}

fn ny_json(sesjon: &str, pane: &str, mappe: &Path, navn: Option<&str>) -> Value {
    json!({ "versjon": VERSJON, "sesjon": sesjon, "pane": pane, "mappe": mappe, "navn": navn })
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

#[cfg(test)]
mod tester {
    use std::time::Duration;

    use super::*;

    fn pane(id: &str) -> Pane {
        // Feltrekkefølgen er `FELT`. Mappa er ikke i et git-repo, så `repo` blir `null`.
        Pane::fra_linje(&format!("$0\ts\t@0\t0\t{id}\tt\t80\t20\t80\t20\t0\t0\t1\t\t1\tvindu\t1700000000000\tzsh\t\t0\t/nonexistent/nerdr")).unwrap()
    }

    fn claude() -> Claude {
        Claude {
            tilstand: Some(Tilstand::Spør),
            endret: 1_700_000_000_000,
            mappe: Some(PathBuf::from("/nonexistent/nerdr")),
            samtale: Some("00000000-0000-0000-0000-000000000000".to_string()),
        }
    }

    fn nøkler(v: &Value) -> Vec<&str> {
        let mut nøkler: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        nøkler.sort();
        nøkler
    }

    #[test]
    fn liste_json_har_kontraktens_felt() {
        let p = pane("%1");
        let claude = HashMap::from([("%1".to_string(), claude())]);
        let rader = [Rad::Pane(&p, Merke::Spør)];

        let v = liste_json(&rader, &claude);

        assert_eq!(nøkler(&v), ["sesjoner", "versjon"]);
        assert_eq!(v["versjon"], 1);
        let s = &v["sesjoner"][0];
        assert_eq!(
            nøkler(s),
            [
                "branch", "endret", "gruppe", "mappe", "merke", "oppgave", "pane", "repo", "samtale", "sesjon", "sett", "tilstand", "vindu",
            ]
        );
        assert_eq!(s["pane"], "%1");
        assert_eq!(s["tilstand"], "venter");
        assert_eq!(s["endret"], 1_700_000_000_000u64);
        assert_eq!(s["sett"], 1_700_000_000_000u64);
        assert!(s["repo"].is_null());
        assert!(s["branch"].is_null());
    }

    #[test]
    fn liste_json_hopper_over_rader_uten_claude() {
        let p = pane("%2");
        let rader = [Rad::Pane(&p, Merke::Spør)];

        let v = liste_json(&rader, &HashMap::new());

        assert_eq!(v["sesjoner"], json!([]));
    }

    #[test]
    fn samtaler_json_har_kontraktens_felt() {
        let s = Samtale {
            id: "00000000-0000-0000-0000-000000000000".to_string(),
            mappe: PathBuf::from("/nonexistent/nerdr"),
            tittel: "Fiks testene".to_string(),
            endret: UNIX_EPOCH + Duration::from_millis(1_700_000_000_000),
        };

        let v = samtaler_json(&[s]);

        assert_eq!(nøkler(&v), ["samtaler", "versjon"]);
        assert_eq!(v["versjon"], 1);
        let e = &v["samtaler"][0];
        assert_eq!(nøkler(e), ["endret", "id", "mappe", "tittel"]);
        assert_eq!(e["endret"], 1_700_000_000_000u64);
        assert_eq!(e["mappe"], "/nonexistent/nerdr");
        assert_eq!(e["tittel"], "Fiks testene");
    }

    #[test]
    fn ny_json_har_kontraktens_felt() {
        let v = ny_json("nerdr", "%3", Path::new("/nonexistent/nerdr"), Some("navn"));

        assert_eq!(nøkler(&v), ["mappe", "navn", "pane", "sesjon", "versjon"]);
        assert_eq!(v["versjon"], 1);
        assert_eq!(v["sesjon"], "nerdr");
        assert_eq!(v["pane"], "%3");
        assert_eq!(v["mappe"], "/nonexistent/nerdr");
        assert_eq!(v["navn"], "navn");
    }

    #[test]
    fn ny_json_navn_kan_være_null() {
        let v = ny_json("nerdr", "%3", Path::new("/nonexistent/nerdr"), None);

        assert!(v["navn"].is_null());
    }
}
