# nerdr

Et CLI som viser Claude-sesjonene dine i tmux i en liste til venstre og strømmer
den valgte live til høyre, så du kan følge og skrive til dem fra ett sted.

## Installere

Krever tmux. Ferdige binærer finnes for macOS (arm64 og Intel) og Linux (x86_64).

```
curl -fsSL https://github.com/nicolayro/nerdr/releases/latest/download/installer.sh | sh
```

nerdr legges i `~/opt/bin`, `~/bin` eller `~/.local/bin`, den første som finnes,
eller der den ligger fra før. `NERDR_BIN=<mappe>` foran `sh` velger en annen, og
`NERDR_VERSJON=v0.1.0` en bestemt versjon. Kjør kommandoen på nytt for å
oppdatere. nerdr oppdaterer seg ikke selv.

Eller be Claude om det: «Installer nerdr etter INSTALL.md i nicolayro/nerdr.»
[INSTALL.md](INSTALL.md) er skrevet for at Claude kan følge den.

## Avinstallere

```
curl -fsSL https://github.com/nicolayro/nerdr/releases/latest/download/avinstaller.sh | sh
```

nerdr lagrer ingenting annet enn seg selv, så det er alt. Har du satt
`NERDR_MAPPER`, kan linja fjernes fra skallets oppstartsfil.

## Bygge fra kildekoden

Krever Rust 1.85 eller nyere.

```
cargo install --locked --git https://github.com/nicolayro/nerdr nerdr
```

nerdr havner i `~/.cargo/bin`, og fjernes med `cargo uninstall nerdr`.

## Bruk

Kjør `nerdr` i en vanlig terminalfane, utenfor tmux. Bare Claude som kjører i
tmux, vises. Har ikke tmux noen sesjon, lager nerdr en.

| Tast | Hva |
| --- | --- |
| `↑` `↓` / `k` `j` | Velg Claude |
| `⏎` | Gå inn i sesjonen |
| `C-q` | Tilbake til lista |
| `n` | Ny Claude i en mappe du velger |
| `C-r` | Fortsett en tidligere samtale i en mappe |
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

`C-r` viser de siste samtalene du har hatt med Claude, med mappe, tittel og
hvor lenge siden. `⏎` fortsetter den du velger, i mappa den hørte til.
Samtaler som kjører, står ikke der. `C-r` i popupen bytter mellom samtalene og
mappene.

## Utvikle

Rust 1.85 eller nyere. `cargo run` starter nerdr, og `scripts/tui-test.sh` kjører
den mot en falsk Claude på egne tmux-servere. `scripts/installer-test.sh` kjører
installeringen og avinstalleringen mot en release i en lokal mappe og et falskt
`HOME`. Når en tag som `v1.0.0` pushes,
bygger `.github/workflows/release.yml` binærene og legger dem i en release.
