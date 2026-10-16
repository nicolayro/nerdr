# nerdr

Et CLI som viser Claude-sesjonene dine i tmux i en liste til venstre og strømmer
den valgte live til høyre, så du kan følge og skrive til dem fra ett sted.

## Installere

Krever tmux og `gh`, logget inn med tilgang til repoet.

```
gh api repos/nicolayro/nerdr/contents/scripts/installer.sh -H 'Accept: application/vnd.github.raw' | sh
```

Eller be Claude om det: «Installer nerdr etter INSTALL.md i nicolayro/nerdr.»
[INSTALL.md](INSTALL.md) er skrevet for at Claude kan følge den, også for å
sette opp mappene og bygge fra kildekoden når det ikke finnes en binær.

## Bruk

Kjør `nerdr` i en vanlig terminalfane, utenfor tmux. Bare Claude som kjører i
tmux, vises. Har ikke tmux noen sesjon, lager nerdr en.

| Tast | Hva |
| --- | --- |
| `↑` `↓` / `k` `j` | Velg Claude |
| `⏎` | Gå inn i sesjonen |
| `C-q` | Tilbake til lista |
| `n` | Ny Claude i en mappe du velger |
| `?` | Hjelp |
| `q` | Avslutt, fra lista |

Du kan også bruke musen. Klikk i lista går inn i en Claude, og klikk og
musehjul i sesjonen går videre til Claude.

Lista er delt i tre. Under **venter på deg** står de som spør om noe (`●`) og de
som er ferdige med noe du ikke har sett (`◆`). Så kommer de som **jobber** (`◐`),
og til slutt de du har **sett** (`○`). En ferdig Claude regnes som sett når du
har vært inne i den.

Hver Claude viser mappen og branchen den jobber i, og under det oppgaven den
holder på med.

`n` åpner en popup der du søker fram en mappe under `~/git`, og `⏎` starter
`claude` der i en tmux-sesjon med samme navn som mappa. Finnes sesjonen
allerede, får den et nytt vindu. Du er inne i den med en gang, og kortet står
med `◌` til Claude er klar. Andre mapper settes kolonseparert i
`NERDR_MAPPER`, som `NERDR_MAPPER=~/git:~/kode`.

## Utvikle

Rust 1.85 eller nyere. `cargo run` starter nerdr, og `scripts/tui-test.sh` kjører
den mot en falsk Claude på egne tmux-servere. Når en tag som `v1.0.0` pushes,
bygger `.github/workflows/release.yml` binærene og legger dem i en release.
