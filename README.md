# nerdr

Et CLI med oversikt over Claude Code-sesjonene dine i en sidebar til venstre, og
den valgte sesjonen strømmet live til høyre. Du kan skrive rett inn i den.

nerdr kjører ikke Claude selv. tmux eier terminalene, og nerdr kobler seg til
som en klient i control mode (`tmux -C`), slik iTerm2 gjør. Sesjonene lever
videre når nerdr lukkes, og alt du har i tmux fra før, virker som før.

## Bygg og kjør

```
cargo build --release
./target/release/nerdr
```

Kjør den i en vanlig terminalfane, utenfor tmux. Inne i tmux virker den også,
men da har du to lag med prefiks-taster.

| Tast | Hva |
| --- | --- |
| `↑` `↓` / `k` `j` | Velg sesjon. Den vises med én gang. |
| `⏎` / `Tab` / klikk | Gå inn i sesjonen. Tastene går da til den. |
| `C-]` | Tilbake til lista. |
| `q` / `C-c` | Avslutt, fra lista. |

`nerdr --json` skriver oversikten i samme format som `sesjoner.py --json` i
sidebar-repoet, uten feltet `aktiv`. `-L <socket>` velger tmux-server, som
`tmux -L`.

## Slik henger det sammen

- `kjerne/` har ingen avhengigheter og vet ikke hvordan noe vises: control
  mode-klienten, sesjonsoversikten, tilstandsdeteksjonen og git-oppslagene. En
  annen klient, for eksempel i GPUI, kan bygges på den samme kjernen.
- `tui/` er ratatui-klienten.

Skjermbildet hentes med `capture-pane -e` over control mode-tilkoblingen når
tmux melder `%output` for panen som vises, høyst 30 ganger i sekundet. tmux
er altså den eneste som tolker terminalstrømmen.

Tilstandsdeteksjonen er oversatt fra `sesjoner.py`. `scripts/sammenlign.py`
kjører begge mot de ekte sesjonene og viser forskjellene felt for felt:

```
python3 -I scripts/sammenlign.py target/debug/nerdr ../sidebar/tmux/bin/sesjoner.py
```

`scripts/tui-test.sh` kjører nerdr mot en falsk Claude på to egne tmux-servere,
uten å røre din.

## Kjente begrensninger

- nerdr velger vinduet den viser (`select-window`). Andre klienter i samme
  sesjon, som en iTerm-fane med `tmux attach`, bytter vindu samtidig.
- Vinduet får størrelsen til nerdr så lenge nerdr er klienten som sist var i
  bruk (`window-size latest`).
- Musa fanges for klikk i sidebaren. Hold `⌥` i iTerm2 for å markere tekst.
- Ingen scrollback ennå, bare det som er på skjermen.
- `Shift+Enter` sendes som `S-Enter`, og når bare fram hvis den ytre terminalen
  melder den og panen har extended keys på.

## Avhengigheter

Bare `ratatui` med `default-features = false` og `crossterm`. Det gir 63
tredjeparts-crates på macOS, 27 av dem fra crossterm.

Alternativer å vurdere:

- **`vt100` og `tui-term`** i stedet for `capture-pane -e`: nerdr tolker
  bytestrømmen fra `%output` selv. Jevnere ved mye output og ingen henting per
  bilde, mot omtrent fem crates til og et skjermbilde som må hentes én gang ved
  oppstart.
- **crossterm uten ratatui**: sidebaren og panen tegnes med en egen liten
  buffer-diff, rundt 150 linjer. Halverer antall crates.
