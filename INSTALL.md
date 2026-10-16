# Installere nerdr

Skrevet for at Claude skal kunne installere og sette opp nerdr for brukeren.
Gå gjennom stegene i rekkefølge, og spør brukeren der det står.

## 1. Sjekk forutsetningene

- `tmux -V` skal gi en versjon. Mangler den, foreslå `brew install tmux`
  (macOS) og spør før du installerer.
- `gh auth status` skal vise at brukeren er logget inn. Repoet
  `nicolayro/nerdr` er privat, og `gh repo view nicolayro/nerdr` skal virke.
  Gir det 404, mangler brukeren tilgang og må be eieren om det. Stopp der.

## 2. Installer binæren

```
gh api repos/nicolayro/nerdr/contents/scripts/installer.sh -H 'Accept: application/vnd.github.raw' | sh
```

Scriptet laster ned siste release for maskinen, sjekker sjekksummen og legger
`nerdr` i `~/.local/bin`. Sett `NERDR_BIN` foran `sh` for en annen mappe.
Sier scriptet at mappa ikke står i `PATH`, legg til linja det foreslår i
skallets oppstartsfil (`~/.zshrc` for zsh). Kjør `nerdr --version` for å se at
det virker.

Finnes det ingen release ennå, eller ingen binær for maskinen, sier scriptet
fra. Da trengs Rust 1.85 eller nyere, og nerdr bygges med

```
CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install --locked --git https://github.com/nicolayro/nerdr nerdr
```

Uten variabelen henter cargo med sin egen git-klient, som ikke kommer til et
privat repo. `cargo` legger nerdr i `~/.cargo/bin`, og den mappa må stå i
`PATH`.

## 3. Sett mappene for nye Claude-sesjoner

`n` i nerdr starter en Claude i en mappe brukeren velger blant mappene rett
under `~/git`. Spør hvor brukeren har kodet sitt. Er det andre steder, sett
`NERDR_MAPPER` i skallets oppstartsfil, kolonseparert:

```
export NERDR_MAPPER=~/git:~/kode
```

## 4. Fortell brukeren hvordan nerdr brukes

- nerdr kjøres i en vanlig terminalfane, utenfor tmux.
- Bare Claude som kjører i tmux, vises. Startes Claude i en vanlig fane, ser
  ikke nerdr den. `n` i nerdr starter en Claude i tmux.
- `?` i nerdr viser tastene. `nerdr --help` viser det samme i terminalen.

## Oppdatere

Kjør kommandoen i steg 2 på nytt. nerdr oppdaterer seg ikke selv.
