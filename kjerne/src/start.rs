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

use crate::kontroll::sitat;
use crate::{paner, tmux};

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

/// Hvordan Claude startes. Uten noe er det en ny samtale uten navn.
#[derive(Clone, Copy, Default)]
pub struct Valg<'a> {
    /// En tidligere samtale som fortsettes.
    pub samtale: Option<&'a str>,
    pub navn: Option<&'a str>,
    /// Den første meldingen.
    pub prompt: Option<&'a str>,
}

/// Det som skrives inn i skallet. Navnet og prompten står i enkle
/// anførselstegn, så skallet ikke tolker noe i dem.
pub fn claude(valg: Valg) -> String {
    let mut ut = String::from("claude");
    if let Some(id) = valg.samtale {
        ut += &format!(" --resume {id}");
    }
    if let Some(navn) = valg.navn {
        ut += &format!(" -n {}", sitat(navn));
    }
    if let Some(prompt) = valg.prompt {
        ut += &format!(" {}", sitat(prompt));
    }
    ut
}

/// Svarer med pane-id-en, eller med en feil når sesjonen finnes.
pub fn ny_sesjon(navn: &str, mappe: &Path) -> Vec<String> {
    let mappe = mappe.to_string_lossy();
    ["new-session", "-d", "-s", navn, "-c", &mappe, "-P", "-F", "#{pane_id}"].map(String::from).to_vec()
}

/// Så nerdr vet at sesjonen er dens egen, og kan lukke den når Claude-en er
/// borte.
pub fn merk(pane: &str) -> Vec<String> {
    ["set-option", "-t", pane, paner::FRA_NERDR, "1"].map(String::from).to_vec()
}

pub fn skriv_inn(pane: &str, claude: &str) -> [Vec<String>; 2] {
    [
        ["send-keys", "-t", pane, "-l", claude].map(String::from).to_vec(),
        ["send-keys", "-t", pane, "Enter"].map(String::from).to_vec(),
    ]
}

/// Starter Claude med enkeltstående tmux-kommandoer, og gir sesjonsnavnet og
/// pane-id-en.
pub fn start(socket: Option<&str>, mappe: &Path, valg: Valg) -> io::Result<(String, String)> {
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
    tmux::kjør(socket, &merk(&pane))?;
    for kommando in skriv_inn(&pane, &claude(valg)) {
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

    /// Kjører kommandoen i `sh` med `printf` i stedet for `claude`, og gir
    /// argumentene slik `claude` ville fått dem.
    fn argumenter(kommando: &str) -> Vec<String> {
        let kommando = kommando.replacen("claude", r"printf '%s\0'", 1);
        let ut = std::process::Command::new("sh").args(["-c", &kommando]).output().unwrap();
        String::from_utf8(ut.stdout).unwrap().split_terminator('\0').map(String::from).collect()
    }

    #[test]
    fn navn_og_prompt_kommer_fram_som_de_er() {
        let prompt = "it's $HOME, `ls` og \"to\"\nlinjer \\ ! *";
        let kommando = claude(Valg { navn: Some("ticket 11 postadresse"), prompt: Some(prompt), ..Valg::default() });
        assert_eq!(argumenter(&kommando), ["-n", "ticket 11 postadresse", prompt]);
    }

    #[test]
    fn en_tidligere_samtale_fortsettes_med_resume() {
        assert_eq!(claude(Valg::default()), "claude");
        assert_eq!(claude(Valg { samtale: Some("abc-123"), ..Valg::default() }), "claude --resume abc-123");
    }

    #[test]
    fn første_forsøk_er_navnet_til_mappa_og_de_neste_får_nummer() {
        assert_eq!(nummerert("api", 1), "api");
        assert_eq!(nummerert("api", 2), "api-2");
    }
}
