//! Tekst som vises likt i lista, i mappevelgeren og fra kommandoene.

use std::path::Path;

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
