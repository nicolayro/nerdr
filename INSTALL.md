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

`n` i nerdr starter en Claude i en mappe brukeren velger blant mappene rett
under `~/git`. Spør hvor brukeren har koden sin. Er det andre steder, sett
`NERDR_MAPPER` i skallets oppstartsfil, kolonseparert:

```
export NERDR_MAPPER=~/git:~/kode
```

## 4. Fortell brukeren hvordan nerdr brukes

- nerdr kjøres i en vanlig terminalfane, utenfor tmux.
- Bare Claude som kjører i tmux, vises. Startes Claude i en vanlig fane, ser
  ikke nerdr den. `n` i nerdr starter en Claude i tmux, og `C-r` fortsetter en
  tidligere samtale.
- `?` i nerdr viser tastene. `nerdr --help` viser det samme i terminalen.

## Oppdatere og fjerne

Kjør kommandoen i steg 2 på nytt for å oppdatere. nerdr oppdaterer seg ikke
selv. For å fjerne nerdr:

```
curl -fsSL https://github.com/nicolayro/nerdr/releases/latest/download/avinstaller.sh | sh
```

Fjern også `NERDR_MAPPER`- og `PATH`-linjene du la til, hvis brukeren vil det.
