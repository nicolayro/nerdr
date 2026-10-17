//! Kjernen i nerdr: tmux control mode, panene på serveren og Claude-ene som
//! kjører i dem. Vet ikke hvordan noe vises, så en annen klient kan bygges på
//! den samme kjernen.

use std::env;
use std::path::PathBuf;

pub mod git;
pub mod kontroll;
pub mod merke;
pub mod oppsett;
pub mod paner;
pub mod register;
pub mod samtaler;
pub mod sgr;
pub mod start;
pub mod tmux;

/// Samme katalog som Claude Code bruker: `$CLAUDE_CONFIG_DIR`, ellers `~/.claude`.
pub fn konfigkatalog() -> PathBuf {
    env::var("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env::var("HOME").unwrap_or_default()).join(".claude"))
}
