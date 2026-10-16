//! En terminal hører til en Claude: et skall i et eget tmux-vindu rett etter
//! vinduet til Claude-en. Panen merkes med pane-id-en til Claude-en i en
//! brukeropsjon, så tmux husker hvem den hører til, også når nerdr ikke kjører.

use std::path::Path;

pub const OPSJON: &str = "@nerdr-claude";

/// Svarer med pane-id-en. Uten `mappe` velger tmux selv.
pub fn ny(claude_vindu: &str, mappe: Option<&Path>) -> Vec<String> {
    let mut kommando = vec!["new-window", "-d", "-a", "-t", claude_vindu];
    let mappe = mappe.map(Path::to_string_lossy);
    if let Some(mappe) = &mappe {
        kommando.extend(["-c", mappe]);
    }
    kommando.extend(["-P", "-F", "#{pane_id}"]);
    kommando.into_iter().map(String::from).collect()
}

pub fn knytt(pane: &str, claude: &str) -> Vec<String> {
    ["set-option", "-p", "-t", pane, OPSJON, claude].map(String::from).to_vec()
}
