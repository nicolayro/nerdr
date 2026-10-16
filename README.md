# nerdr

En TUI for oversikt over og navigasjon mellom Claude-sesjonene dine.

## Forutsetninger

- [Claude Code](https://claude.com/claude-code)
- tmux (testet med 3.8)
- macOS eller Linux

## Installere

```
curl -fsSL https://github.com/nicolayro/nerdr/releases/latest/download/installer.sh | sh
```

- nerdr legges i `~/opt/bin`, `~/bin` eller `~/.local/bin`, den første som
  finnes, eller der den ligger fra før.
- `NERDR_BIN=<mappe>` foran `sh` velger en annen mappe, og
  `NERDR_VERSJON=v0.1.0` en bestemt versjon.
- Kjør kommandoen på nytt for å oppdatere. nerdr oppdaterer seg ikke selv.

Du kan også be Claude om å installere og sette opp nerdr etter
[INSTALL.md](INSTALL.md).

## Bruk

Kjør `nerdr` i en vanlig terminalfane, utenfor tmux. Bare Claude som kjører i
tmux, vises. Har ikke tmux noen sesjon, lager nerdr en.

| Tast | Hva |
| --- | --- |
| `↑` `↓` / `k` `j` | Velg Claude |
| `⏎` | Gå inn i sesjonen |
| `C-q` | Tilbake til lista |
| `t` | Terminal i mappa til Claude-en |
| `n` | Ny Claude i en mappe du velger |
| `C-r` | Fortsett en tidligere samtale |
| `?` | Hjelp |
| `q` | Avslutt, fra lista |

Klikk i lista går inn i en Claude, og klikk og musehjul i sesjonen går videre
til Claude.

Hver Claude viser mappen og branchen den jobber i, og oppgaven under. Lista er
delt i tre:

| | |
| --- | --- |
| **venter på deg** | `●` spør om noe, `◆` er ferdig med noe du ikke har sett |
| **jobber** | `◐` jobber, `◌` starter |
| **sett** | `○` er ferdig, og du har vært inne i den siden |

### Starte og fortsette

`n` åpner en popup der du søker fram en mappe, og `⏎` starter `claude` der i en
tmux-sesjon med samme navn som mappa. Finnes sesjonen, får den et nytt vindu.

`C-r` viser de siste samtalene dine med mappe, tittel og alder, og `⏎`
fortsetter den du velger, i mappa den hørte til. Samtaler som kjører, står ikke
der. `C-r` i popupen bytter mellom samtalene og mappene.

I begge tilfeller er du inne i den nye Claude-en med en gang.

### Terminal

`t` går inn i terminalen til Claude-en du har valgt, og lager den første gang.
Den er et skall i et eget tmux-vindu rett etter Claude-vinduet, i samme mappe,
og står under Claude-en i lista med programmet som kjører. Avslutter du skallet,
lukkes vinduet, og du er tilbake i lista. Terminalen kjører videre når nerdr
avslutter, og står der igjen neste gang.

## Kommandoer

For script og agenter, ved siden av TUI-et. Med `--json` kommer svaret som JSON
med et `versjon`-felt.

| | |
| --- | --- |
| `nerdr liste` | Claude-ene i tmux, med tilstand (`venter`, `jobber` eller `klar`), pane, mappe, branch og oppgave |
| `nerdr samtaler` | Tidligere samtaler, nyest først, med id, mappe og tittel |
| `nerdr ny <mappe>` | Start en Claude i mappa, som `n` |
| `nerdr ny --fortsett <id>` | Fortsett en samtale i mappa den hørte til, som `C-r` |

Hvilke Claude-er du har sett, husker bare TUI-et, så `liste` skiller ikke en
som er ferdig, fra en du har sett.

## Innstillinger

| | |
| --- | --- |
| `NERDR_MAPPER` | Mappene `n` søker i, kolonseparert, som `~/git:~/kode`. Standard er `~/git`. |
| `CLAUDE_CONFIG_DIR` | Katalogen til Claude Code, som for Claude Code selv. Standard er `~/.claude`. |
| `nerdr -L <socket>` | tmux-serveren, som `tmux -L`. |

## Avinstallere

```
curl -fsSL https://github.com/nicolayro/nerdr/releases/latest/download/avinstaller.sh | sh
```

nerdr lagrer ingenting annet enn seg selv. Har du satt `NERDR_MAPPER`, kan linja
fjernes fra skallets oppstartsfil.

## Bygge fra kildekoden

Krever Rust 1.85 eller nyere.

```
cargo install --locked --git https://github.com/nicolayro/nerdr nerdr
```

nerdr havner i `~/.cargo/bin`, og fjernes med `cargo uninstall nerdr`.

## Utvikle

- `cargo run` starter nerdr.
- `scripts/tui-test.sh` kjører nerdr mot en falsk Claude på egne tmux-servere.
- `scripts/installer-test.sh` kjører installeringen og avinstalleringen mot en
  release i en lokal mappe og et falskt `HOME`.
- En tag som `v1.0.0` får `.github/workflows/release.yml` til å bygge binærene
  og legge dem i en release.
