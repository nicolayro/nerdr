//! Tilstanden til Claude-sesjonene, lest fra registeret Claude Code selv
//! skriver i `~/.claude/sessions/<pid>.json`.
//!
//! Formatet er internt i Claude Code og ikke dokumentert. Felt som mangler
//! eller har ukjente verdier, gir ingen tilstand framfor en feil.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tilstand {
    Venter,
    Jobber,
    Klar,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Oppføring {
    /// `sesjon:@vindu.%pane`, satt når sesjonen kjører i tmux.
    tmux: Option<String>,
    status: Option<String>,
    #[serde(default)]
    updated_at: u64,
}

/// Pane-id → tilstand for hver Claude som kjører i tmux.
pub fn les(katalog: &Path) -> HashMap<String, Tilstand> {
    let mut nyest: HashMap<String, (u64, Tilstand)> = HashMap::new();
    let Ok(filer) = fs::read_dir(katalog) else { return HashMap::new() };
    for fil in filer.flatten() {
        let sti = fil.path();
        if sti.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Some((pane, oppdatert, tilstand)) = fs::read_to_string(&sti).ok().and_then(|t| tolk(&t)) else {
            continue;
        };
        // En sesjon som krasjet, lar fila ligge igjen, og pane-id-en kan være
        // tatt i bruk på nytt. Den sist oppdaterte vinner.
        if nyest.get(&pane).is_none_or(|(t, _)| oppdatert > *t) {
            nyest.insert(pane, (oppdatert, tilstand));
        }
    }
    nyest.into_iter().map(|(pane, (_, tilstand))| (pane, tilstand)).collect()
}

fn tolk(tekst: &str) -> Option<(String, u64, Tilstand)> {
    let o: Oppføring = serde_json::from_str(tekst).ok()?;
    let tmux = o.tmux?;
    let (_, pane) = tmux.rsplit_once('.')?;
    let tilstand = match o.status?.as_str() {
        "waiting" => Tilstand::Venter,
        "busy" => Tilstand::Jobber,
        "idle" => Tilstand::Klar,
        _ => return None,
    };
    Some((pane.to_string(), o.updated_at, tilstand))
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn leser_pane_og_tilstand() {
        let t = r#"{"pid":1,"tmux":"sidebar:@7.%37","status":"waiting","waitingFor":"dialog open","updatedAt":5}"#;
        assert_eq!(tolk(t), Some(("%37".into(), 5, Tilstand::Venter)));
    }

    #[test]
    fn uten_tmux_eller_med_ukjent_status_gir_ingenting() {
        assert_eq!(tolk(r#"{"status":"busy"}"#), None);
        assert_eq!(tolk(r#"{"tmux":"a:@1.%2","status":"ny-verdi"}"#), None);
        assert_eq!(tolk("ikke json"), None);
    }

    #[test]
    fn sist_oppdaterte_vinner_når_panen_er_delt() {
        let katalog = std::env::temp_dir().join(format!("nerdr-register-{}", std::process::id()));
        fs::create_dir_all(&katalog).unwrap();
        fs::write(katalog.join("1.json"), r#"{"tmux":"a:@1.%2","status":"idle","updatedAt":10}"#).unwrap();
        fs::write(katalog.join("2.json"), r#"{"tmux":"b:@3.%2","status":"busy","updatedAt":20}"#).unwrap();
        fs::write(katalog.join("2.key"), "hemmelig").unwrap();

        assert_eq!(les(&katalog), HashMap::from([("%2".to_string(), Tilstand::Jobber)]));
        fs::remove_dir_all(katalog).unwrap();
    }
}
