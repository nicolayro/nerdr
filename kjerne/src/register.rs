//! Claude-ene som kjører, og tilstanden deres, lest fra registeret Claude Code selv
//! skriver i `~/.claude/sessions/<pid>.json`.
//!
//! Formatet er internt i Claude Code og ikke dokumentert. Felt som mangler
//! eller har ukjente verdier, gir ingen tilstand framfor en feil.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tilstand {
    Spør,
    Jobber,
    Klar,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claude {
    /// `None` når Claude melder en status nerdr ikke kjenner.
    pub tilstand: Option<Tilstand>,
    /// Millisekunder siden 1970.
    pub endret: u64,
    pub mappe: Option<PathBuf>,
    /// Samtalen som kjører, som i `claude --resume <id>`.
    pub samtale: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Oppføring {
    /// `sesjon:@vindu.%pane`, satt når Claude kjører i tmux.
    tmux: Option<String>,
    status: Option<String>,
    #[serde(default)]
    updated_at: u64,
    status_updated_at: Option<u64>,
    cwd: Option<PathBuf>,
    session_id: Option<String>,
}

/// Pane-id → Claude for hver Claude som kjører i tmux.
pub fn les(katalog: &Path) -> HashMap<String, Claude> {
    let mut nyest: HashMap<String, (u64, Claude)> = HashMap::new();
    let Ok(filer) = fs::read_dir(katalog) else { return HashMap::new() };
    for fil in filer.flatten() {
        let sti = fil.path();
        if sti.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Some((pane, oppdatert, claude)) = fs::read_to_string(&sti).ok().and_then(|t| tolk(&t)) else {
            continue;
        };
        // En Claude som krasjet, lar fila ligge igjen, og pane-id-en kan være
        // tatt i bruk på nytt. Den sist oppdaterte vinner.
        if nyest.get(&pane).is_none_or(|(t, _)| oppdatert > *t) {
            nyest.insert(pane, (oppdatert, claude));
        }
    }
    nyest.into_iter().map(|(pane, (_, claude))| (pane, claude)).collect()
}

fn tolk(tekst: &str) -> Option<(String, u64, Claude)> {
    let o: Oppføring = serde_json::from_str(tekst).ok()?;
    let tmux = o.tmux?;
    let (_, pane) = tmux.rsplit_once('.')?;
    let tilstand = match o.status.as_deref() {
        Some("waiting") => Some(Tilstand::Spør),
        Some("busy") => Some(Tilstand::Jobber),
        // `shell` er ferdig ved prompten med et skall som kjører i bakgrunnen.
        Some("idle" | "shell") => Some(Tilstand::Klar),
        _ => None,
    };
    let endret = o.status_updated_at.unwrap_or(o.updated_at);
    Some((pane.to_string(), o.updated_at, Claude { tilstand, endret, mappe: o.cwd, samtale: o.session_id }))
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn leser_pane_og_tilstand() {
        let t = r#"{"pid":1,"tmux":"nerdr:@7.%37","status":"waiting","cwd":"/a/nerdr","updatedAt":5,"statusUpdatedAt":3,"sessionId":"abc"}"#;
        let claude =
            Claude { tilstand: Some(Tilstand::Spør), endret: 3, mappe: Some("/a/nerdr".into()), samtale: Some("abc".into()) };
        assert_eq!(tolk(t), Some(("%37".into(), 5, claude)));
        let uten = r#"{"tmux":"a:@1.%2","status":"idle","updatedAt":5}"#;
        assert_eq!(tolk(uten).map(|(_, _, s)| s.endret), Some(5));
    }

    #[test]
    fn uten_tmux_gir_ingenting() {
        assert_eq!(tolk(r#"{"status":"busy"}"#), None);
        assert_eq!(tolk("ikke json"), None);
    }

    #[test]
    fn ukjent_status_gir_claude_uten_tilstand() {
        let tilstand = |t| tolk(t).map(|(_, _, c)| c.tilstand);
        assert_eq!(tilstand(r#"{"tmux":"a:@1.%2","status":"ny-verdi"}"#), Some(None));
        assert_eq!(tilstand(r#"{"tmux":"a:@1.%2"}"#), Some(None));
        assert_eq!(tilstand(r#"{"tmux":"a:@1.%2","status":"shell"}"#), Some(Some(Tilstand::Klar)));
    }

    #[test]
    fn sist_oppdaterte_vinner_når_panen_er_delt() {
        let katalog = std::env::temp_dir().join(format!("nerdr-register-{}", std::process::id()));
        fs::create_dir_all(&katalog).unwrap();
        fs::write(katalog.join("1.json"), r#"{"tmux":"a:@1.%2","status":"idle","updatedAt":10}"#).unwrap();
        fs::write(katalog.join("2.json"), r#"{"tmux":"b:@3.%2","status":"busy","updatedAt":20}"#).unwrap();
        fs::write(katalog.join("2.key"), "hemmelig").unwrap();

        let jobber = Claude { tilstand: Some(Tilstand::Jobber), endret: 20, mappe: None, samtale: None };
        assert_eq!(les(&katalog), HashMap::from([("%2".to_string(), jobber)]));
        fs::remove_dir_all(katalog).unwrap();
    }
}
