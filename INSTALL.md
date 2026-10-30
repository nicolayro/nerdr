# Installere nerdr

Skrevet for at Claude skal kunne installere og sette opp nerdr for brukeren.
Gå gjennom stegene i rekkefølge, og spør brukeren der det står.

## 1. Sjekk forutsetningene

`tmux -V` skal gi en versjon. Mangler den, si at nerdr trenger tmux, og spør
om installeringen i steg 2 kan installere den. Scriptet kan ikke spørre selv
når du kjører det, så sett `NERDR_TMUX=ja` foran `sh` hvis brukeren sier ja.

## 2. Installer binæren

```
curl -fsSL https://github.com/nicolayro/nerdr/releases/latest/download/installer.sh | sh
```

Uten `NERDR_TMUX=ja` skriver scriptet bare kommandoen som installerer tmux.
`sudo` på Linux spør etter passordet, så den kan brukeren måtte kjøre selv,
med `! <kommando>`.

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

## 3. Mappene for nye Claude-sesjoner

«+ ny sesjon» nederst i lista i nerdr starter en Claude i en mappe brukeren
velger. Velgeren søker i mappene brukeren har brukt Claude i, og i mappene
under `~`, så det trengs ikke noe oppsett. Vil brukeren bare søke noen steder,
sett `NERDR_MAPPER` i skallets oppstartsfil, kolonseparert:

```
export NERDR_MAPPER=~/git:~/kode
```

## 4. Fortell brukeren hvordan nerdr brukes

- nerdr kjøres i en vanlig terminalfane, utenfor tmux.
- Tastene går til Claude-en brukeren er inne i. nerdr styres med knappene
  nederst i lista, eller med prefikset fra tmux (`C-b` hvis ikke noe annet er
  satt) og en tast.
- Bare Claude som kjører i tmux, vises. Startes Claude i en vanlig fane, ser
  ikke nerdr den. «+ ny sesjon», eller prefikset og `f`, starter en Claude i
  tmux, og prefikset og `r` fortsetter en tidligere samtale.
- «hjelp» nederst i lista, eller prefikset og `?`, viser tastene.
  `nerdr --help` viser det samme i terminalen.
- nerdr avsluttes med `C-c` fra lista, med prefikset og `d`, eller med
  «avslutt» nederst i lista. Claude-ene kjører videre i tmux.
- Spør brukeren senere hvordan noe gjøres i nerdr, kjør `nerdr forklar`. Den
  sier hvordan det gjøres med knappene og med tastene.
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
| se en Claude som mangler i lista | Den kjører ikke i tmux. Avslutt den, og fortsett samtalen med prefikset og `r` i nerdr. |
| ha en mappe i velgeren | Mapper under `~` er med, utenom skjulte mapper og det som er bygget. Står `NERDR_MAPPER` satt, sett mappa over den der. Eller start en Claude der med `nerdr ny <mappe>` og skriv noe til den, så står mappa i velgeren etter det. |
| et annet prefiks | nerdr bruker prefikset fra `~/.tmux.conf`, ellers `C-b`. `NERDR_PREFIKS=C-a` endrer bare nerdr. |
| endre utseendet | Prefikset og `b`. |
| beholde utseendet og bredden etter at tmux er stoppet | Valgene står i `@nerdr-utseende` og `@nerdr-liste`, som forsvinner med tmux-serveren. Les verdiene med `tmux show-options -gqv @nerdr-utseende`, og legg dem i `~/.tmux.conf`, som `set -g @nerdr-utseende "farge=av plass=minimal"` og `set -g @nerdr-liste 46`. |
| tilbake til standardutseendet | `tmux set-option -gu @nerdr-utseende`, og fjern linja i `~/.tmux.conf` hvis den finnes. |

## Oppdatere og fjerne

Kjør kommandoen i steg 2 på nytt for å oppdatere.
nerdr oppdaterer seg ikke selv. For å fjerne nerdr:

```
curl -fsSL https://github.com/nicolayro/nerdr/releases/latest/download/avinstaller.sh | sh
```

Fjern også `NERDR_MAPPER`- og `PATH`-linjene du la til, hvis brukeren vil det.
