# Installere nerdr

Skrevet for at Claude skal kunne installere og sette opp nerdr for brukeren.
Gå gjennom stegene i rekkefølge, og spør brukeren der det står.

## 1. Sjekk forutsetningene

`tmux -V` skal gi en versjon. Mangler den, foreslå `brew install tmux` (macOS)
og spør før du installerer.

## 2. Installer binæren

```
curl -fsSL https://github.com/nicolayro/nerdr/releases/latest/download/installer.sh | sh
```

Scriptet laster ned siste release for maskinen, sjekker sjekksummen og legger
`nerdr` i `~/opt/bin`, `~/bin` eller `~/.local/bin`, den første som finnes, eller
der den ligger fra før. Vil brukeren ha den et annet sted, sett `NERDR_BIN=<mappe>`
foran `sh`. Sier scriptet at mappa ikke står i `PATH`, spør om du kan legge til
linja det foreslår i skallets oppstartsfil (`~/.zshrc` for zsh). Kjør
`nerdr --version` for å se at det virker.

Sier scriptet at det ikke finnes en binær for maskinen, bygg fra kildekoden med
Rust 1.85 eller nyere:

```
cargo install --locked --git https://github.com/nicolayro/nerdr nerdr
```

## 3. Sett mappene for nye Claude-sesjoner

`n` i nerdr starter en Claude i en mappe brukeren velger. Velgeren viser
mappene brukeren har brukt Claude i, og mappene rett under `~/git`. Spør hvor
brukeren har koden sin. Er det ikke `~/git`, sett `NERDR_MAPPER` i skallets
oppstartsfil, kolonseparert:

```
export NERDR_MAPPER=~/git:~/kode
```

## 4. Fortell brukeren hvordan nerdr brukes

- nerdr kjøres i en vanlig terminalfane, utenfor tmux.
- Bare Claude som kjører i tmux, vises. Startes Claude i en vanlig fane, ser
  ikke nerdr den. `n` i nerdr starter en Claude i tmux, og `C-r` fortsetter en
  tidligere samtale.
- `?` i nerdr viser tastene. `nerdr --help` viser det samme i terminalen.
- nerdr avsluttes med `q` fra lista, med prefikset og `d` fra panen, eller
  med «avslutt» nederst i lista. Claude-ene kjører videre i tmux.
- Du kan selv bruke `nerdr liste`, `nerdr samtaler` og `nerdr ny` med
  `--json`, for eksempel for å svare på hvilke Claude-er som venter på
  brukeren, eller for å fortsette en samtale.

## Hjelpe brukeren med oppsettet senere

nerdr er en klient til tmux. tmux kjører terminalene og Claude-ene, og nerdr
viser dem. nerdr lagrer ingenting selv. Det den husker, står i tmux-optioner,
og resten er miljøvariabler i skallets oppstartsfil. `nerdr --help` viser
tastene og hvilket prefiks og hvilke mapper som gjelder.

| Brukeren vil | Svar |
| --- | --- |
| stoppe alt | `tmux kill-server`. Spør først, for det stopper alle Claude-er og terminaler i tmux. |
| se en Claude som mangler i lista | Den kjører ikke i tmux. Avslutt den, og fortsett samtalen med `C-r` i nerdr. |
| ha en mappe i velgeren | Sett mappa over den i `NERDR_MAPPER`. Eller start en Claude der med `nerdr ny <mappe>` og skriv noe til den, så står mappa i velgeren etter det. |
| et annet prefiks | nerdr bruker prefikset fra `~/.tmux.conf`, ellers `C-b`. `NERDR_PREFIKS=C-a` endrer bare nerdr. |
| endre utseendet | Prefikset og `b`. |
| beholde utseendet og bredden etter at tmux er stoppet | Valgene står i `@nerdr-utseende` og `@nerdr-liste`, som forsvinner med tmux-serveren. Les verdiene med `tmux show-options -gqv @nerdr-utseende`, og legg dem i `~/.tmux.conf`, som `set -g @nerdr-utseende "farge=mye plass=tett"` og `set -g @nerdr-liste 46`. |
| tilbake til standardutseendet | `tmux set-option -gu @nerdr-utseende`, og fjern linja i `~/.tmux.conf` hvis den finnes. |

## Oppdatere og fjerne

Kjør kommandoen i steg 2 på nytt for å oppdatere. nerdr oppdaterer seg ikke
selv. For å fjerne nerdr:

```
curl -fsSL https://github.com/nicolayro/nerdr/releases/latest/download/avinstaller.sh | sh
```

Fjern også `NERDR_MAPPER`- og `PATH`-linjene du la til, hvis brukeren vil det.
