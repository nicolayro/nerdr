//! Det nerdr kan, vist med `?` og `nerdr --help`.

use crate::grupper::Merke;
use crate::ny;

pub const OM: &str = "Claude-sesjonene dine i tmux. Den du velger i lista, vises til høyre, \
og du kan gå inn i den og skrive.";

pub const TASTER: [(&str, &str); 8] = [
    ("↑↓  k j", "velg Claude"),
    ("⏎", "gå inn i sesjonen"),
    ("C-q", "tilbake til lista"),
    ("t", "terminal i mappa til Claude-en"),
    ("n", "ny Claude i en mappe du velger"),
    ("C-r", "fortsett en tidligere samtale"),
    ("?", "denne hjelpen"),
    ("q", "avslutt"),
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
