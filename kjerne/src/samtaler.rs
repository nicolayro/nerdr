//! Tidligere samtaler, lest fra transkriptene Claude Code skriver i
//! `~/.claude/projects/<mappe>/<id>.jsonl`.
//!
//! Formatet er internt og ikke dokumentert. Claude Code skriver tittelen og
//! mappa på nytt nær slutten av fila etter hver tur, så bare halen leses.
//! Filene kan bli flere megabyte.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Deserialize;

const HALE: u64 = 256 * 1024;

#[derive(Debug, PartialEq)]
pub struct Samtale {
    pub id: String,
    pub mappe: PathBuf,
    pub tittel: String,
    pub endret: SystemTime,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Linje {
    #[serde(rename = "type")]
    type_: Option<String>,
    custom_title: Option<String>,
    ai_title: Option<String>,
    last_prompt: Option<String>,
    cwd: Option<PathBuf>,
}

/// De `antall` sist endrede samtalene i `katalog` (`~/.claude/projects`),
/// nyest først, uten dem i `utenom`. Samtaler uten en mappe som finnes, tas
/// ikke med, siden de ikke kan fortsettes der, og heller ikke samtaler uten
/// en eneste melding.
pub fn siste(katalog: &Path, antall: usize, utenom: &HashSet<String>) -> Vec<Samtale> {
    let mut filer: Vec<(SystemTime, String, PathBuf)> = fs::read_dir(katalog)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|prosjekt| fs::read_dir(prosjekt.path()).ok())
        .flatten()
        .flatten()
        .filter_map(|fil| {
            let sti = fil.path();
            let id = sti.file_name()?.to_str()?.strip_suffix(".jsonl")?.to_string();
            let endret = fil.metadata().ok()?.modified().ok()?;
            Some((endret, id, sti))
        })
        .filter(|(_, id, _)| !utenom.contains(id))
        .collect();
    filer.sort_by(|a, b| b.0.cmp(&a.0));

    filer.into_iter().filter_map(|(endret, id, sti)| samtale(&sti, id, endret)).take(antall).collect()
}

/// Samtalen med `id`, uansett hvor gammel den er.
pub fn finn(katalog: &Path, id: &str) -> Option<Samtale> {
    // Id-en blir en del av en sti.
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    fs::read_dir(katalog).ok()?.flatten().find_map(|prosjekt| {
        let sti = prosjekt.path().join(format!("{id}.jsonl"));
        let endret = fs::metadata(&sti).ok()?.modified().ok()?;
        samtale(&sti, id.to_string(), endret)
    })
}

fn samtale(sti: &Path, id: String, endret: SystemTime) -> Option<Samtale> {
    let (mappe, tittel) = les(sti)?;
    mappe.is_dir().then_some(Samtale { id, mappe, tittel, endret })
}

fn les(sti: &Path) -> Option<(PathBuf, String)> {
    let mut fil = File::open(sti).ok()?;
    let lengde = fil.metadata().ok()?.len();
    let fra = lengde.saturating_sub(HALE);
    fil.seek(SeekFrom::Start(fra)).ok()?;
    let mut bytes = Vec::new();
    fil.read_to_end(&mut bytes).ok()?;
    let tekst = String::from_utf8_lossy(&bytes);
    // Midt i fila er første linje bare en bit, og den hoppes over.
    let tekst = if fra > 0 { tekst.split_once('\n').map_or("", |(_, r)| r) } else { &tekst };
    tolk(tekst)
}

/// Tittelen er den brukeren har gitt, ellers den Claude har laget, ellers
/// siste melding. Den siste av hver gjelder.
fn tolk(tekst: &str) -> Option<(PathBuf, String)> {
    let (mut mappe, mut egen, mut ai, mut melding) = (None, None, None, None);
    for linje in tekst.lines().filter_map(|l| serde_json::from_str::<Linje>(l).ok()) {
        mappe = linje.cwd.or(mappe);
        match linje.type_.as_deref() {
            Some("custom-title") => egen = linje.custom_title.or(egen),
            Some("ai-title") => ai = linje.ai_title.or(ai),
            Some("last-prompt") => melding = linje.last_prompt.or(melding),
            _ => {}
        }
    }
    let melding = melding.and_then(|m| m.lines().map(str::trim).find(|l| !l.is_empty()).map(str::to_string));
    Some((mappe?, egen.or(ai).or(melding)?))
}

#[cfg(test)]
mod tester {
    use super::*;
    use std::time::Duration;

    #[test]
    fn egen_tittel_vinner_over_den_claude_laget() {
        let t = r#"{"type":"user","cwd":"/a/nerdr","message":{}}
{"type":"ai-title","aiTitle":"Første"}
{"type":"custom-title","customTitle":"Min tittel"}
{"type":"ai-title","aiTitle":"Andre"}"#;
        assert_eq!(tolk(t), Some(("/a/nerdr".into(), "Min tittel".into())));
    }

    #[test]
    fn uten_tittel_brukes_første_linje_av_siste_melding() {
        let t = "{\"type\":\"last-prompt\",\"lastPrompt\":\"\\n  Fiks testene\\nog mer\",\"cwd\":\"/a\"}";
        assert_eq!(tolk(t), Some(("/a".into(), "Fiks testene".into())));
    }

    #[test]
    fn uten_mappe_eller_uten_meldinger_gir_ingenting() {
        assert_eq!(tolk(r#"{"type":"ai-title","aiTitle":"Uten mappe"}"#), None);
        assert_eq!(tolk(r#"{"type":"user","cwd":"/a"}
{"type":"last-prompt"}"#), None);
    }

    #[test]
    fn nyest_først_uten_dem_som_kjører() {
        let rot = std::env::temp_dir().join(format!("nerdr-samtaler-{}", std::process::id()));
        let prosjekt = rot.join("-a-nerdr");
        fs::create_dir_all(&prosjekt).unwrap();
        let mappe = rot.display().to_string();
        let skriv = |id: &str, tittel: &str, sekunder: u64| {
            let sti = prosjekt.join(format!("{id}.jsonl"));
            let linjer = format!("{{\"cwd\":\"{mappe}\"}}\n{{\"type\":\"ai-title\",\"aiTitle\":\"{tittel}\"}}\n");
            fs::write(&sti, linjer).unwrap();
            let tid = SystemTime::UNIX_EPOCH + Duration::from_secs(sekunder);
            File::options().write(true).open(&sti).unwrap().set_modified(tid).unwrap();
        };
        skriv("gammel", "Gammel", 100);
        skriv("ny", "Ny", 300);
        skriv("kjører", "Kjører", 200);

        let titler: Vec<_> = siste(&rot, 10, &HashSet::from(["kjører".to_string()]))
            .into_iter()
            .map(|s| s.tittel)
            .collect();
        assert_eq!(titler, ["Ny", "Gammel"]);
        assert_eq!(finn(&rot, "gammel").map(|s| s.tittel).as_deref(), Some("Gammel"));
        assert_eq!(finn(&rot, "../-a-nerdr/ny"), None);
        fs::remove_dir_all(rot).unwrap();
    }
}
