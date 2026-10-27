//! Å starte en Claude i tmux: i en sesjon med navnet til mappa, eller i et nytt
//! vindu i den når sesjonen finnes.
//!
//! Vinduet starter med et skall, og `claude` skrives inn i det. Kjørt av tmux
//! går den gjennom `sh -c` eller `zsh -c`, som ikke leser skallets
//! oppstartsfil, og en `claude`-funksjon der ville blitt hoppet over.

use std::io;
use std::path::Path;

use crate::tmux;

/// Punktum og kolon er skilletegn i tmux-targets, og tmux bytter dem ut selv.
/// Da ville navnet ikke funnet sesjonen igjen.
pub fn sesjonsnavn(mappe: &Path) -> String {
    let navn = mappe.file_name().map_or_else(|| "claude".into(), |n| n.to_string_lossy());
    navn.replace(['.', ':'], "-")
}

/// Det som skrives inn: en ny samtale, eller en tidligere som fortsettes.
pub fn claude(samtale: Option<&str>) -> String {
    match samtale {
        Some(id) => format!("claude --resume {id}"),
        None => "claude".into(),
    }
}

/// Svarer med pane-id-en, eller med en feil når sesjonen finnes.
pub fn ny_sesjon(navn: &str, mappe: &Path) -> Vec<String> {
    let mappe = mappe.to_string_lossy();
    ["new-session", "-d", "-s", navn, "-c", &mappe, "-P", "-F", "#{pane_id}"].map(String::from).to_vec()
}

/// Svarer med pane-id-en.
pub fn nytt_vindu(navn: &str, mappe: &Path) -> Vec<String> {
    let (mål, mappe) = (format!("={navn}:"), mappe.to_string_lossy());
    ["new-window", "-t", &mål, "-c", &mappe, "-P", "-F", "#{pane_id}"].map(String::from).to_vec()
}

pub fn skriv_inn(pane: &str, claude: &str) -> [Vec<String>; 2] {
    [
        ["send-keys", "-t", pane, "-l", claude].map(String::from).to_vec(),
        ["send-keys", "-t", pane, "Enter"].map(String::from).to_vec(),
    ]
}

/// Starter Claude med enkeltstående tmux-kommandoer, og gir sesjonsnavnet og
/// pane-id-en.
pub fn start(socket: Option<&str>, mappe: &Path, samtale: Option<&str>) -> io::Result<(String, String)> {
    let navn = sesjonsnavn(mappe);
    let pane = match tmux::kjør(socket, &ny_sesjon(&navn, mappe)) {
        Ok(pane) => pane,
        Err(feil) if feil.kind() == io::ErrorKind::NotFound => return Err(feil),
        Err(_) => tmux::kjør(socket, &nytt_vindu(&navn, mappe))?,
    };
    for kommando in skriv_inn(&pane, &claude(samtale)) {
        tmux::kjør(socket, &kommando)?;
    }
    Ok((navn, pane))
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn sesjonsnavn_uten_skilletegn() {
        assert_eq!(sesjonsnavn(Path::new("/h/git/nit.æ")), "nit-æ");
        assert_eq!(sesjonsnavn(Path::new("/h/git/a:b")), "a-b");
        assert_eq!(sesjonsnavn(Path::new("/")), "claude");
    }

    #[test]
    fn nytt_vindu_treffer_bare_sesjonen_med_akkurat_det_navnet() {
        assert_eq!(nytt_vindu("api", Path::new("/a"))[2], "=api:");
    }
}
