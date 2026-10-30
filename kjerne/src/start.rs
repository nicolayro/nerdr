//! Å starte en Claude i tmux, i en ny sesjon med navnet til mappa. Finnes den,
//! får den nye et nummer etter navnet, `nerdr-2`. Med en sesjon hver hører
//! vinduene du lager ved siden av en Claude, til den, og ikke til alle som
//! jobber i samme mappe.
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

/// En feil som ikke er at navnet er tatt, kommer igjen for hvert navn, og
/// skal ikke gi kommandoer til tmux i det uendelige.
pub const MAKS_FORSØK: u32 = 100;

/// Navnet i forsøk nummer `forsøk`, som starter på 1.
pub fn nummerert(navn: &str, forsøk: u32) -> String {
    if forsøk <= 1 { navn.into() } else { format!("{navn}-{forsøk}") }
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

pub fn skriv_inn(pane: &str, claude: &str) -> [Vec<String>; 2] {
    [
        ["send-keys", "-t", pane, "-l", claude].map(String::from).to_vec(),
        ["send-keys", "-t", pane, "Enter"].map(String::from).to_vec(),
    ]
}

/// Starter Claude med enkeltstående tmux-kommandoer, og gir sesjonsnavnet og
/// pane-id-en.
pub fn start(socket: Option<&str>, mappe: &Path, samtale: Option<&str>) -> io::Result<(String, String)> {
    let mappenavn = sesjonsnavn(mappe);
    let mut forsøk = 1;
    let (navn, pane) = loop {
        let navn = nummerert(&mappenavn, forsøk);
        match tmux::kjør(socket, &ny_sesjon(&navn, mappe)) {
            Ok(pane) => break (navn, pane),
            Err(feil) if feil.kind() == io::ErrorKind::NotFound || forsøk >= MAKS_FORSØK => return Err(feil),
            Err(_) => forsøk += 1,
        }
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
    fn første_forsøk_er_navnet_til_mappa_og_de_neste_får_nummer() {
        assert_eq!(nummerert("api", 1), "api");
        assert_eq!(nummerert("api", 2), "api-2");
    }
}
