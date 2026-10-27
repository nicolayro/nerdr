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
| `:` | Handlinger: alt nerdr kan gjøre, søkbart |
| `↑` `↓` / `k` `j` | Velg Claude |
| `⏎` | Gå inn i sesjonen |
| `n` `+` | Ny sesjon eller terminal |
| `C-r` | Hent en tidligere samtale |
| `?` | Hjelp |
| `q` | Avslutt, fra lista |

Nederst i lista står «avslutt» med tasten som virker der du er: `q` fra lista,
og prefikset og `d` fra panen. Claude-ene kjører videre i tmux når nerdr
avslutter, og er der neste gang du starter nerdr. `tmux kill-server` stopper
tmux og alt som kjører i det.

`:` åpner handlingene, og fra panen er det prefikset og `:`. Du kan også
klikke på «handlinger» til høyre på streken med fanene. Der står alt nerdr kan
gjøre, med tasten ved siden av, og du søker fram det du vil og trykker `⏎`.
Valgene i konfigurasjonen står der også, som «mye farge» og «skjul tiden».

Et klikk på en Claude i lista velger den, og et dobbeltklikk går inn i den. Et
klikk i panen går også inn, og et klikk i lista tar deg ut igjen. Klikk og
musehjul i sesjonen går videre når programmet der har slått på mus, som Claude
Code i fullskjerm. Ellers ruller hjulet i historikken, og en tast tar deg
tilbake til bunnen. Det du drar over, kopieres når du slipper, med `pbcopy`,
`wl-copy`, `xclip` eller `xsel`. I `less` og `man` blir hjulet til piltaster.

Er vinduet delt i flere paner, viser nerdr dem der tmux har plassert dem, i
hver sin boks eller med streker mellom. Tastene går til den aktive panen, og et klikk gjør en annen pane aktiv. En
terminal ved siden av Claude-en får du ved å dele vinduet eller lage et nytt.
Øverst står vinduene i sesjonen som faner, på en strek som fortsetter streken
øverst i lista, og vinduet som vises, har bakgrunn. Et klikk på en fane viser vinduet. Fra lista går du alltid inn i vinduet til Claude-en.

Prefikset er det du setter i `NERDR_PREFIKS`, ellers det samme som i
tmux-oppsettet ditt, ellers `C-b`. Tastene etter det er de samme som i tmux, og
virker både fra lista og fra panen:

| Etter prefikset | Hva |
| --- | --- |
| `:` | Handlinger: alt nerdr kan gjøre, søkbart |
| `f` | Ny sesjon, som i tmux-sessionizer |
| `r` | Hent en tidligere samtale |
| `c` | Ny terminal, i mappa til panen |
| `n` `p` | Neste og forrige vindu |
| `0`–`9` | Vinduet med det nummeret |
| `l` | Siste vindu, det du var i før |
| `&` | Lukk vindu, etter at du har svart `y` |
| `%` | Del side om side |
| `"` | Del over og under |
| `o` / `←` `↑` `↓` `→` | Neste pane, eller panen i den retningen |
| `;` | Siste pane, den som var aktiv før |
| `z` | Zoom pane, inn og ut |
| `x` | Lukk pane, etter at du har svart `y` |
| `{` `}` | Flytt pane bakover og framover |
| `Space` | Neste oppsett |
| `C-o` | Roter paner |
| `C-←` `C-↑` `C-↓` `C-→` | Endre størrelsen, med `M-` fem om gangen |
| `PgUp` | Rull en side opp |
| `(` `)` | Forrige og neste Claude |
| `b` | Konfigurasjon |
| `w` `s` | Tilbake til lista |
| `d` | Avslutt |
| `?` | Hjelp |
| prefikset | Send prefikset til panen |

Hver Claude står i en boks, med tegnet, repoet og tiden i toppkanten, oppgaven
inni og branchen under. Et worktree står med navnet til repoet det er laget fra. Lista er delt i tre:

| | |
| --- | --- |
| **venter** | `●` spør om noe, `◆` er ferdig med noe du ikke har sett |
| **jobber** | `◐` jobber, `◌` starter |
| **ferdig** | `○` er ferdig, og du har vært inne i den siden |

I hver gruppe står den som kom sist inn, øverst. Det står hvor lenge den har
ventet, jobbet eller vært ferdig. Får ikke lista plass, ruller den med
musehjulet og følger den du velger. Pilene øverst og nederst sier hvor mange
som er over og under, og om noen over venter på deg. Et klikk på en pil ruller
en side.

Dra i streken mellom lista og panen for å gjøre lista bredere eller smalere.
Drar du den helt inn, blir lista en stripe med bare tegnene, og et dobbeltklikk
på streken bytter mellom stripa og bredden du hadde. Bredden står i den globale
tmux-optionen `@nerdr-liste`, så den er den samme neste gang.

Når du sist var inne i en Claude, skriver nerdr i pane-optionen `@nerdr-sett`
i tmux. Den forsvinner med panen, og det en Claude blir ferdig med mens nerdr
er lukket, venter på deg neste gang.

### Starte og fortsette

`n` eller `+` åpner en meny med alt nerdr kan lage, og med musen «+ ny»
nederst i lista. Fra panen har hvert valg sin tast etter prefikset. Det som
ikke har en egen mappe, legges i mappa og sesjonen til panen som vises:

| | | Etter prefikset |
| --- | --- | --- |
| `n` | Ny sesjon: Claude i en mappe du velger | `f` |
| `r` | Hent samtale, som `C-r` | `r` |
| `t` | Ny terminal i et nytt vindu, som `+` etter fanene | `c` |
| `%` `"` | Del vinduet side om side eller over og under | `%` `"` |

`n` i menyen åpner en popup der du søker fram en mappe, og `⏎` starter `claude`
der i en tmux-sesjon med samme navn som mappa. Finnes sesjonen, får den et nytt
vindu. Først står mappene du har brukt Claude i, sist brukt først, så du ikke
trenger å sette noe. Så kommer mappene rett under `NERDR_MAPPER` og prosjektene
under dem, inntil fire nivåer ned, som appene i et monorepo. Et prosjekt er en mappe med
`package.json`, `pom.xml`, `build.gradle`, `Cargo.toml`, `go.mod`,
`pyproject.toml` eller `deno.json`, utenfor `node_modules`, `target`, `build`
og testmapper. Prosjektene kommer i lista når de er lest, og til da står det
`…` etter antallet.

`C-r` viser de siste samtalene dine med mappe, tittel og alder, og `⏎`
fortsetter den du velger, i mappa den hørte til. Samtaler som kjører, står ikke
der. `C-r` i popupen bytter mellom samtalene og mappene.

I begge tilfeller er du inne i den nye Claude-en med en gang.

## Kommandoer

For script og agenter, ved siden av TUI-et. Med `--json` kommer svaret som JSON
med et `versjon`-felt.

| | |
| --- | --- |
| `nerdr liste` | Claude-ene i tmux, gruppert som i TUI-et, med pane, mappe, repo, branch og oppgave |
| `nerdr samtaler` | Tidligere samtaler, nyest først, med id, mappe og tittel |
| `nerdr ny <mappe>` | Start en Claude i mappa, som «ny sesjon» i menyen |
| `nerdr ny --fortsett <id>` | Fortsett en samtale i mappa den hørte til, som `C-r` |

I JSON fra `liste` er `tilstand` det Claude melder selv (`venter`, `jobber`
eller `klar`), eller `null` når nerdr ikke kjenner statusen. `merke` er det lista viser (`spør`, `ulest`, `jobber` eller
`sett`), og `gruppe` er der den står (`venter`, `jobber` eller `ferdig`).
`sett` er da du sist var inne i Claude-en i nerdr, i millisekunder som
`endret`, eller `null` når nerdr ikke har sett den.

## Konfigurasjon

Prefikset og `b` åpner konfigurasjonen. Valget gjelder med en gang, og tmux
husker det i `@nerdr-utseende`.

| | |
| --- | --- |
| **paner** | `bokser` gir hver pane en boks med avrundede hjørner og luft mellom, også når vinduet har én pane. `linjer` lar panene fylle visningen med streker mellom, som i tmux. |
| **farge** | `lite` har farge bare på tegnet. `middels` gir også overskriftene farge. `mye` gir rammene og strekene en tone av fargen til gruppen. |
| **plass** | `tett` har én linje for hver Claude, uten boks, med tegnet, oppgaven og tiden. `romslig` har mer luft mellom gruppene. |
| **tid** | Om det står hvor lenge den har ventet, jobbet eller vært ferdig. |
| **orden** | `nerdr` deler lista i venter, jobber og ferdig. `mappe` har en gruppe for hvert repo, i alfabetisk rekkefølge, og Claude-ene i hver står der de er i tmux. `aktivitet` har ingen grupper, og den som sist endret tilstand, står øverst. |
| **tittel** | Hvor mange linjer oppgaven får i boksen, fra `1` til `3`. |
| **uthev** | Hvordan repoet i toppkanten står ut: `ingen` er dempet, `fet` er fet, og `farge` har fargen til gruppen. |
| **symbol** | Om tegnet står til `venstre` for repoet eller til `høyre` for tiden. |

Standard er middels farge, vanlig plass, tiden synlig, ordnet som nerdr, én
linje til oppgaven, fet repo og tegnet til venstre. `tittel`, `uthev` og
`symbol` gjelder ikke tett. `nerdr liste` er alltid ordnet som nerdr.

## Innstillinger

| | |
| --- | --- |
| `NERDR_MAPPER` | Mappene mappevelgeren viser mappene under, i tillegg til dem du har brukt Claude i. Kolonseparert, som `~/git:~/kode`. Standard er `~/git`. |
| `NERDR_PREFIKS` | Prefikset, skrevet som i tmux: `C-a`, `M-a` eller `C-Space`. Standard er prefikset i tmux-oppsettet ditt, ellers `C-b`. Det endrer ikke tmux, bare nerdr. |
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
