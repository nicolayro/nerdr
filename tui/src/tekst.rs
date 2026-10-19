//! Tekst som vises likt i lista, i mappevelgeren og fra kommandoene.

use std::fs;
use std::path::{Path, PathBuf};

use nerdr_kjerne::git::Repo;
use nerdr_kjerne::paner::Pane;

/// Kort nok til å stå ytterst på en linje i et kort.
pub fn varighet(ms: u64) -> String {
    match ms / 60_000 {
        0 => "<1m".into(),
        m @ 1..60 => format!("{m}m"),
        m @ 60..1440 => format!("{}t", m / 60),
        m => format!("{}d", m / 1440),
    }
}

/// Navnet på mappa, eller hele stien når den ikke har noe, som `/`.
pub fn mappenavn(mappe: &Path) -> String {
    mappe.file_name().map_or_else(|| mappe.display().to_string(), |n| n.to_string_lossy().into())
}

/// Stien med `~` for hjemmemappa. tmux gir stien uten symlenker, så
/// hjemmemappa prøves også slik.
pub fn med_tilde(mappe: &Path) -> String {
    let Some(hjem) = std::env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from) else {
        return mappe.display().to_string();
    };
    let ekte = fs::canonicalize(&hjem).ok();
    match [Some(&hjem), ekte.as_ref()].into_iter().flatten().find_map(|h| mappe.strip_prefix(h).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~/{}", rest.display()),
        None => mappe.display().to_string(),
    }
}

/// Hvor en Claude jobber: repoet, mappa utenfor git, eller vinduet i tmux når
/// mappa er ukjent.
pub fn sted(pane: &Pane, mappe: Option<&Path>, repo: Option<&Repo>) -> String {
    match (repo, mappe) {
        (Some(repo), _) => repo.navn.clone(),
        (None, Some(mappe)) => mappenavn(mappe),
        (None, None) => format!("{}:{}", pane.sesjon, pane.vindu),
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn varighet_i_minutter_timer_og_døgn() {
        assert_eq!(varighet(59_000), "<1m");
        assert_eq!(varighet(14 * 60_000), "14m");
        assert_eq!(varighet(3 * 3_600_000 + 60_000), "3t");
        assert_eq!(varighet(2 * 86_400_000), "2d");
    }

    #[test]
    fn mappenavn_er_siste_del_av_stien() {
        assert_eq!(mappenavn(Path::new("/h/git/nerdr")), "nerdr");
        assert_eq!(mappenavn(Path::new("/")), "/");
    }
}
