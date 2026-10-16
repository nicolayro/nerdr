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
| `n` | Ny Claude i en mappe du velger |
| `C-r` | Fortsett en tidligere samtale |
| `?` | Hjelp |
| `q` | Avslutt, fra lista |

Klikk i lista går inn i en Claude. Klikk og musehjul i sesjonen går videre når
programmet der har slått på mus, som Claude Code i fullskjerm. Ellers ruller
hjulet i historikken, og en tast tar deg tilbake til bunnen. I `less` og `man`
blir hjulet til piltaster.

Er vinduet delt i flere paner, viser nerdr dem der tmux har plassert dem, i
hver sin boks eller med streker mellom, etter `NERDR_RAMMER`. Tastene går til den aktive panen, og et klikk gjør en annen pane aktiv. En
terminal ved siden av Claude-en får du ved å dele vinduet, og med zoom fyller
Claude-en visningen igjen mens terminalen kjører videre.

Prefikset er det samme som i tmux-oppsettet ditt, ellers `C-b`. Tastene etter
det er de samme som i tmux, og virker både fra lista og fra panen:

| Etter prefikset | Hva |
| --- | --- |
| `%` | Del vinduet side om side |
| `"` | Del vinduet over og under |
| `o` / `←` `↑` `↓` `→` | Neste pane, eller panen i den retningen |
| `;` | Forrige aktive pane |
| `z` | Zoom inn og ut på panen |
| `x` | Lukk panen, etter at du har svart `y` |
| `{` `}` | Bytt plass med panen før eller etter |
| `Space` | Neste oppsett |
| `C-o` | Roter panene |
| `C-←` `C-↑` `C-↓` `C-→` | Endre størrelsen, med `M-` fem om gangen |
| `PgUp` | Rull en side opp |
| `n` `p` | Neste og forrige i lista |
| `w` `s` | Tilbake til lista |
| `d` | Avslutt |
| `?` | Hjelp |
| prefikset | Send prefikset til panen |

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
| `NERDR_RAMMER` | `bokser` gir hver pane en boks med avrundede hjørner og luft mellom, også når vinduet har én pane. `linjer` lar panene fylle visningen med streker mellom, som i tmux. Standard er `bokser`. |
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
