//! Det nerdr kan, vist med `?` og `nerdr --help`.

use nerdr_kjerne::merke::Merke;

use crate::grupper::glyf;
use crate::ny;
use crate::taster::{PREFIKS_VARIABEL, Prefiks};

pub const OM: &str = "Claude-sesjonene dine i tmux. Den du velger i lista, vises til høyre, \
og du kan gå inn i den og skrive.";

pub const TASTER: [(&str, &str); 8] = [
    (":", "handlinger: alt nerdr kan gjøre"),
    ("↑↓  k j", "velg Claude"),
    ("⏎", "gå inn i sesjonen"),
    ("klikk", "velg Claude, dobbeltklikk går inn"),
    ("n  +", "ny sesjon eller terminal"),
    ("C-r", "hent samtale"),
    ("?", "denne hjelpen"),
    ("q", "avslutt"),
];

/// Etter prefikset, fra lista og fra panen.
pub const PREFIKSTASTER: [(&str, &str); 24] = [
    (":", "handlinger: alt nerdr kan gjøre"),
    ("f", "ny sesjon"),
    ("r", "hent samtale"),
    ("c", "ny terminal"),
    ("n p", "neste og forrige vindu"),
    ("0-9", "vinduet med det nummeret"),
    ("l", "siste vindu"),
    ("&", "lukk vindu"),
    ("%", "del side om side"),
    ("\"", "del over og under"),
    ("o ←↑↓→", "neste pane, eller i en retning"),
    (";", "siste pane"),
    ("z", "zoom pane"),
    ("x", "lukk pane"),
    ("{ }", "flytt pane bakover og framover"),
    ("Space", "neste oppsett"),
    ("C-o", "roter paner"),
    ("C-←↑↓→", "endre størrelse, med M- fem om gangen"),
    ("PgUp", "rull en side opp"),
    ("( )", "forrige og neste Claude"),
    ("b", "konfigurasjon"),
    ("w s", "tilbake til lista"),
    ("d", "avslutt"),
    ("prefiks", "send prefikset til panen"),
];

pub const MERKER: [(Merke, &str); 5] = [
    (Merke::Spør, "spør om noe"),
    (Merke::Ulest, "ferdig med noe du ikke har sett"),
    (Merke::Jobber, "jobber"),
    (Merke::Sett, "ferdig, og du har sett den"),
    (Merke::Starter, "starter"),
];

pub const BARE_TMUX: &str = "Bare Claude som kjører i tmux, vises.";

pub fn mapper() -> String {
    format!("Mappene er de du har brukt, og i {} ({}).", ny::mapper(), ny::MAPPER_VARIABEL)
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
    ut += &match Prefiks::fra_miljø() {
        Some(prefiks) => format!("\nEtter prefikset, {} fra {PREFIKS_VARIABEL}:\n", prefiks.navn()),
        None => format!("\nEtter prefikset fra tmux, som C-b, eller fra {PREFIKS_VARIABEL}:\n"),
    };
    for (tast, hva) in PREFIKSTASTER {
        ut += &format!("  {tast:<9}{hva}\n");
    }
    ut += "\nMerker:\n";
    for (merke, hva) in MERKER {
        ut += &format!("  {}  {hva}\n", glyf(merke));
    }
    ut += &format!(
        "\n{BARE_TMUX}\n{}\nFlere mapper settes kolonseparert, som {}=~/git:~/kode.\n",
        mapper(),
        ny::MAPPER_VARIABEL
    );
    ut
}
