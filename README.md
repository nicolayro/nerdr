# nerdr

Lista over tmux-panene dine til venstre, og den valgte panen strømmet live til
høyre. Du kan skrive rett inn i den.

nerdr kjører ikke Claude selv. tmux eier terminalene, og nerdr kobler seg til
som en klient i control mode (`tmux -C`), slik iTerm2 gjør. Panene lever videre
når nerdr lukkes.

## Bygg og kjør

```
cargo run --release
```

Kjør den i en vanlig terminalfane, utenfor tmux.

| Tast | Hva |
| --- | --- |
| `↑` `↓` / `k` `j` | Velg pane. Den vises med én gang. |
| `⏎` | Gå inn i panen. Tastene går da til den. |
| `C-]` | Tilbake til lista. |
| `q` / `C-c` | Avslutt, fra lista. |

`-L <socket>` velger tmux-server, som `tmux -L`. `scripts/tui-test.sh` bruker
det til å kjøre nerdr mot en falsk Claude på to egne tmux-servere.

## Slik henger det sammen

- `kjerne/` har ingen avhengigheter: control mode-klienten, panene fra
  `list-panes` og tolkningen av farger fra `capture-pane -e`.
- `tui/` er ratatui-klienten.

Skjermbildet hentes med `capture-pane -e` når tmux melder `%output` for panen
som vises, høyst 30 ganger i sekundet. Claude Code tømmer og tegner hele
skjermen i hver oppdatering, pakket i en synkronisert oppdatering
(`\033[?2026h` … `\033[?2026l`), så nerdr venter til den er ferdig før den
henter.

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

Branchen `full-prototype` har en første versjon av det som kommer etter hvert:
tilstand per Claude (venter, jobber, klar) oversatt fra `sesjoner.py`,
gruppering på repo og worktree, `--json`, mus og innliming.
