//! Det nerdr kan, vist med `?` og `nerdr --help`.

use crate::grupper::Merke;
use crate::ny;

pub const OM: &str = "Claude-sesjonene dine i tmux. Den du velger i lista, vises til høyre, \
og du kan gå inn i den og skrive.";

pub const TASTER: [(&str, &str); 7] = [
    ("↑↓  k j", "velg Claude"),
    ("⏎", "gå inn i sesjonen"),
    ("C-q", "tilbake til lista"),
    ("n", "ny Claude i en mappe du velger"),
    ("  C-r", "fortsett en tidligere samtale i mappa"),
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
         bruk: nerdr [-L socket]\n\n  \
         -L socket      tmux-serveren, som tmux -L\n  \
         -h, --help     denne hjelpen\n  \
         -V, --version  versjonen\n\n\
         Kjør nerdr i en vanlig terminalfane, utenfor tmux.\n\nTaster:\n",
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
