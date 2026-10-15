# Notater

## Slik henger det sammen

nerdr kjører ikke Claude selv. tmux eier terminalene, og nerdr kobler seg til
som en klient i control mode (`tmux -C`), slik iTerm2 gjør.

- `kjerne/` har ingen avhengigheter: control mode-klienten, panene fra
  `list-panes` og tolkningen av farger fra `capture-pane -e`.
- `tui/` er ratatui-klienten.

Skjermbildet hentes med `capture-pane -e` når tmux melder `%output` for panen
som vises, høyst 30 ganger i sekundet. Claude Code tømmer og tegner hele
skjermen i hver oppdatering, pakket i en synkronisert oppdatering
(`\033[?2026h` … `\033[?2026l`), så nerdr venter til den er ferdig før den
henter. nerdr pakker også sine egne tegninger på samme måte.

`-L <socket>` velger tmux-server, som `tmux -L`. `scripts/tui-test.sh` bruker
det til å kjøre nerdr mot en falsk Claude på to egne tmux-servere.

## Kjente begrensninger

- nerdr velger vinduet den viser. Andre klienter i samme sesjon bytter vindu
  samtidig, og vinduet får størrelsen til nerdr.
- Innliming kommer fram som enkelttaster, så et linjeskift sender inn.
- Ingen scrollback, bare det som er på skjermen.

## Avhengigheter

Bare `ratatui` med `default-features = false` og `crossterm`: 63
tredjeparts-crates på macOS, 27 av dem fra crossterm.

Alternativer å vurdere:

- **`vt100` og `tui-term`** i stedet for `capture-pane -e`: nerdr tolker
  bytestrømmen fra `%output` selv. Omtrent fem crates til.
- **crossterm uten ratatui**: en egen liten buffer-diff, rundt 150 linjer.
  Halverer antall crates.

## Senere iterasjoner

Branchen `full-prototype` har en første versjon av tilstand per Claude
(venter, jobber, klar) oversatt fra `sesjoner.py` i sidebar-repoet,
gruppering på repo og worktree, `--json`, mus og innliming.
