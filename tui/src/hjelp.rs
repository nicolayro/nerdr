//! Det nerdr kan, vist med `?` og `nerdr --help`.

use crate::grupper::Merke;
use crate::ny;

pub const OM: &str = "Claude-sesjonene dine i tmux. Den du velger i lista, vises til høyre, \
og du kan gå inn i den og skrive.";

pub const TASTER: [(&str, &str); 6] = [
    ("↑↓  k j", "velg Claude"),
    ("⏎", "gå inn i sesjonen"),
    ("n", "ny Claude i en mappe du velger"),
    ("C-r", "fortsett en tidligere samtale"),
    ("?", "denne hjelpen"),
    ("q", "avslutt"),
];

/// Etter prefikset, fra lista og fra panen.
pub const PREFIKSTASTER: [(&str, &str); 20] = [
    ("c", "nytt vindu i mappa til panen"),
    ("n p", "neste og forrige vindu"),
    ("0-9", "vinduet med det nummeret"),
    ("l", "forrige vindu du var i"),
    ("&", "lukk vinduet"),
    ("%", "del vinduet side om side"),
    ("\"", "del vinduet over og under"),
    ("o ←↑↓→", "neste pane, eller i en retning"),
    (";", "forrige aktive pane"),
    ("z", "zoom inn og ut på panen"),
    ("x", "lukk panen"),
    ("{ }", "bytt plass med panen før eller etter"),
    ("Space", "neste oppsett"),
    ("C-o", "roter panene"),
    ("C-←↑↓→", "endre størrelse, med M- fem om gangen"),
    ("PgUp", "rull en side opp"),
    ("( )", "forrige og neste Claude i lista"),
    ("w s", "tilbake til lista"),
    ("d", "avslutt"),
    ("prefiks", "send prefikset til panen"),
];

pub const MERKER: [(Merke, &str); 5] = [
    (Merke::Spør, "spør om noe"),
    (Merke::Ferdig, "ferdig med noe du ikke har sett"),
    (Merke::Jobber, "jobber"),
    (Merke::Sett, "sett"),
    (Merke::Starter, "starter"),
];

pub const BARE_TMUX: &str = "Bare Claude som kjører i tmux, vises.";

pub fn mapper() -> String {
    format!("n søker i {} ({}).", ny::mapper(), ny::MAPPER_VARIABEL)
}

pub fn tekst() -> String {
    let mut ut = format!(
        "nerdr {}\n{OM}\n\n\
         bruk: nerdr [-L socket]                 TUI-et, i en vanlig terminalfane utenfor tmux\n       \
         nerdr liste [--json]              Claude-ene i tmux, med tilstand og pane\n       \
         nerdr samtaler [--json]           tidligere samtaler, nyest først, med id\n       \
         nerdr ny <mappe> [--json]         start en Claude i mappa\n       \
         nerdr ny --fortsett <id> [mappe]  fortsett en samtale, i mappa den hørte til\n\n  \
         -L socket      tmux-serveren, som tmux -L\n  \
         --json         JSON i stedet for tekst, med et versjonsfelt\n  \
         -h, --help     denne hjelpen\n  \
         -v, --version  versjonen\n\n\
         Taster i TUI-et:\n",
        env!("CARGO_PKG_VERSION")
    );
    for (tast, hva) in TASTER {
        ut += &format!("  {tast:<9}{hva}\n");
    }
    ut += "\nEtter prefikset fra tmux, som C-b:\n";
    for (tast, hva) in PREFIKSTASTER {
        ut += &format!("  {tast:<9}{hva}\n");
    }
    ut += "\nMerker:\n";
    for (merke, hva) in MERKER {
        ut += &format!("  {}  {hva}\n", merke.glyf());
    }
    ut += &format!(
        "\n{BARE_TMUX}\n{}\nFlere mapper settes kolonseparert, som {}=~/git:~/kode.\n",
        mapper(),
        ny::MAPPER_VARIABEL
    );
    ut
}
