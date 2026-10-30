//! Det nerdr kan, vist med `?` og `nerdr --help`.

use nerdr_kjerne::merke::Merke;
use ratatui::crossterm::event::KeyCode;

use crate::grupper::glyf;
use crate::ny;
use crate::taster::{PREFIKS_VARIABEL, Prefiks};

pub const OM: &str = "Claude-sesjonene dine i tmux. Den du velger i lista, vises til høyre, \
og du kan gå inn i den og skrive.";

/// I lista. Alt annet er etter prefikset, eller en knapp.
pub const TASTER: [(&str, &str); 6] = [
    ("↑↓  k j", "flytt i lista"),
    ("⏎", "vis sesjonen og gå inn i den"),
    ("←→  h l", "brett prosjektet sammen og ut"),
    ("esc", "tilbake dit du var"),
    ("klikk", "flytt dit, dobbeltklikk går inn"),
    ("C-c", "avslutt"),
];

/// Etter prefikset, fra lista og fra panen.
pub const PREFIKSTASTER: [(&str, &str); 26] = [
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
    ("[ ]", "kopimodus: hjkl w b e, v og V marker, y kopier"),
    ("( )", "forrige og neste sesjon"),
    ("b", "konfigurasjon"),
    ("w s", "tilbake til lista"),
    ("d", "avslutt"),
    ("?", "hjelp"),
    ("prefiks", "send prefikset til panen"),
];

/// Tastene etter prefikset, som de er i nerdr, og hva de gjør.
pub type Tast = (&'static [KeyCode], &'static str);

/// Det du trenger først, i `?`. Resten står i handlingene bak `:`.
pub const OVERSIKT: [(&str, &[Tast]); 3] = [
    (
        "Sesjoner",
        &[
            (&[KeyCode::Char('f')], "ny sesjon"),
            (&[KeyCode::Char('r')], "hent samtale"),
            (&[KeyCode::Char('('), KeyCode::Char(')')], "forrige og neste"),
            (&[KeyCode::Char('w')], "tilbake til lista"),
        ],
    ),
    (
        "Panes",
        &[
            (&[KeyCode::Char('%'), KeyCode::Char('"')], "del side om side, over og under"),
            (&[KeyCode::Left, KeyCode::Up, KeyCode::Down, KeyCode::Right], "gå til panen i den retningen"),
            (&[KeyCode::Char('z')], "zoom"),
            (&[KeyCode::Char('x')], "lukk"),
        ],
    ),
    (
        "Windows",
        &[
            (&[KeyCode::Char('c')], "ny terminal"),
            (&[KeyCode::Char('n'), KeyCode::Char('p')], "neste og forrige"),
            (&[KeyCode::Char('&')], "lukk"),
        ],
    ),
];

pub const MERKER: [(Merke, &str); 5] = [
    (Merke::Spør, "spør om noe"),
    (Merke::Ulest, "ferdig, og venter på deg"),
    (Merke::Jobber, "jobber"),
    (Merke::Sett, "ledig, du har gått ut av den"),
    (Merke::Starter, "starter"),
];

pub const BARE_TMUX: &str = "Bare Claude som kjører i tmux, vises.";

pub fn mapper() -> String {
    format!("Mappene er de du har brukt, og i {} ({}).", ny::mapper(), ny::MAPPER_VARIABEL)
}

pub fn tekst() -> String {
    let mut ut = format!(
        "nerdr {}\n{OM}\n\n\
         Er du en agent som skal forklare nerdr for brukeren, hjelpe dem med å gjøre\n\
         noe i nerdr eller melde fra om en feil? Kjør `nerdr forklar` først.\n\n\
         bruk: nerdr [-L socket]                 TUI-et, i en vanlig terminalfane utenfor tmux\n       \
         nerdr forklar                     hvordan nerdr brukes, skrevet for en agent\n       \
         nerdr liste [--json]              Claude-ene i tmux, med tilstand og pane\n       \
         nerdr samtaler [--json]           tidligere samtaler, nyest først, med id\n       \
         nerdr ny <mappe> [--json]         start en Claude i mappa\n       \
         nerdr ny --fortsett <id> [mappe]  fortsett en samtale, i mappa den hørte til\n\n  \
         -L socket      tmux-serveren, som tmux -L\n  \
         --json         JSON i stedet for tekst, med et versjonsfelt\n  \
         -h, --help     denne hjelpen\n  \
         -v, --version  versjonen\n\n\
         I lista:\n",
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
