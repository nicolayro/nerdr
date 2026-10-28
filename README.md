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

Tastene går til Claude-en du er inne i. nerdr selv styrer du med knappene,
eller med prefikset og en tast, som i tmux. I lista er det bare disse:

| Tast | Hva |
| --- | --- |
| `↑` `↓` / `k` `j` | Velg Claude |
| `⏎` | Gå inn i Claude-en |
| `C-c` | Avslutt |

Andre bokstaver i lista gjør ingenting.

Nederst i lista står knappene «+ ny sesjon», «handlinger», «hjelp» og
«avslutt», hver med tasten etter prefikset som gjør det samme. I et lavt vindu og
på stripa står bare «+ ny sesjon» der. Claude-ene
kjører videre i tmux når nerdr avslutter, og er der neste gang du starter
nerdr. `tmux kill-server` stopper tmux og alt som kjører i det.

«handlinger», eller prefikset og `:`, viser alt nerdr kan gjøre, med tasten ved siden av, og du søker fram det du vil og trykker `⏎`.
Valgene i konfigurasjonen står der også, som «mye farge» og «skjul tiden».

Et klikk på en Claude i lista velger den, og et dobbeltklikk går inn i den. Et
klikk i panen går også inn, og et klikk i lista tar deg ut igjen. Står musen
over et kort, kommer `×` der tiden står. Den lukker vinduet til Claude-en, med
det du har åpnet ved siden av, etter at du har svart. Deler flere Claude-er
vinduet, lukkes bare panen. Klikk og
musehjul i sesjonen går videre når programmet der har slått på mus, som Claude
Code i fullskjerm. Ellers ruller hjulet i historikken, og en tast tar deg
tilbake til bunnen. Det du drar over, kopieres når du slipper, med `pbcopy`,
`wl-copy`, `xclip` eller `xsel`. I `less` og `man` blir hjulet til piltaster.

Er vinduet delt i flere paner, viser nerdr dem der tmux har plassert dem, i
hver sin boks eller med streker mellom. Tastene går til den aktive panen, og et klikk gjør en annen pane aktiv. En
terminal ved siden av Claude-en får du ved å dele vinduet eller lage et nytt.
Øverst står vinduene i sesjonen som faner med bakgrunn, og vinduet som vises,
har sterkere bakgrunn. Et klikk på en fane viser vinduet, og `×` i fanen
lukker det etter at du har svart. Fra lista går du alltid inn i vinduet til Claude-en.

Prefikset er det du setter i `NERDR_PREFIKS`, ellers det samme som i
tmux-oppsettet ditt, ellers `C-b`. Tastene etter det er de samme som i tmux, og
virker både fra lista og fra panen:

| Etter prefikset | Hva |
| --- | --- |
| `:` | Handlinger: alt nerdr kan gjøre, søkbart |
| `f` | Ny sesjon |
| `r` | Hent en tidligere samtale |
| `c` | Ny terminal, i mappa til panen |
| `n` `p` | Neste og forrige vindu |
| `0`–`9` | Vinduet med det nummeret |
| `l` | Siste vindu, det du var i før |
| `&` | Lukk vindu, etter at du har svart |
| `%` | Del side om side |
| `"` | Del over og under |
| `o` / `←` `↑` `↓` `→` | Neste pane, eller panen i den retningen |
| `;` | Siste pane, den som var aktiv før |
| `z` | Zoom pane, inn og ut |
| `x` | Lukk pane, etter at du har svart |
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
inni og branchen under. Et worktree står med navnet til repoet det er laget fra. Lista er delt i to:

| | |
| --- | --- |
| **aktive** | `●` spør om noe, `◆` er ferdig og venter på deg, `◐` jobber, `◌` starter |
| **ledige** | `○` er ferdig, og du har gått ut av den uten å sette den i gang igjen |

Den du er inne i, er aldri ledig, og kortet har bakgrunn så du ser hvor du er.
Blant de aktive står den som sist ble aktiv, øverst, og blant de ledige den du
sist gikk ut av. Det står hvor lenge den har ventet, jobbet eller vært ferdig.
Får ikke lista plass, ruller den med musehjulet og følger den du velger. Pilene
øverst og nederst sier at det er flere over og under, og pila øverst er
oransje når noen over venter på deg. Et klikk på en pil ruller en side.

Dra i streken mellom lista og panen for å gjøre lista bredere eller smalere.
Drar du den helt inn, blir lista en stripe med bare tegnene, og et dobbeltklikk
på streken bytter mellom stripa og bredden du hadde. Bredden står i den globale
tmux-optionen `@nerdr-liste`, så den er den samme neste gang.

Når du sist var inne i en Claude, skriver nerdr i pane-optionen `@nerdr-sett`
i tmux. Den forsvinner med panen, og det en Claude blir ferdig med mens nerdr
er lukket, venter på deg neste gang.

### Starte og fortsette

«+ ny sesjon» nederst i lista, eller prefikset og `f`, åpner en popup der du
søker fram en mappe, og `⏎` starter `claude`
der i en tmux-sesjon med samme navn som mappa. Finnes sesjonen, får den et nytt
vindu. Først står mappene du har brukt Claude i, sist brukt først, så du ikke
trenger å sette noe. Så kommer mappene rett under `NERDR_MAPPER` og prosjektene
under dem, inntil fire nivåer ned, som appene i et monorepo. Et prosjekt er en mappe med
`package.json`, `pom.xml`, `build.gradle`, `Cargo.toml`, `go.mod`,
`pyproject.toml` eller `deno.json`, utenfor `node_modules`, `target`, `build`
og testmapper. Prosjektene kommer i lista når de er lest, og til da står det
`…` etter antallet.

Prefikset og `r`, eller «hent samtale» under handlingene, viser de siste samtalene dine med mappe, tittel og alder, og `⏎`
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
| `nerdr ny <mappe>` | Start en Claude i mappa, som «+ ny sesjon» |
| `nerdr ny --fortsett <id>` | Fortsett en samtale i mappa den hørte til, som «hent samtale» |

I JSON fra `liste` er `tilstand` det Claude melder selv (`venter`, `jobber`
eller `klar`), eller `null` når nerdr ikke kjenner statusen. `merke` er det lista viser (`spør`, `ulest`, `jobber` eller
`sett`), og `gruppe` er `venter` for `spør` og `ulest`, `jobber` for `jobber`
og `ferdig` for `sett`.
`sett` er da du sist var inne i Claude-en i nerdr, i millisekunder som
`endret`, eller `null` når nerdr ikke har sett den.

## Konfigurasjon

Prefikset og `b` åpner konfigurasjonen. Valget gjelder med en gang, og tmux
husker det i `@nerdr-utseende`.

| | |
| --- | --- |
| **paner** | `bokser` gir hver pane en boks med avrundede hjørner og luft mellom, også når vinduet har én pane. `linjer` lar panene fylle visningen med streker mellom, som i tmux. |
| **farge** | `på` gir tegnene og tiden fargen til gruppen, og rammene en tone av den. `av` har ingen farge. |
| **plass** | `vanlig` har en boks for hver Claude. `minimal` har én linje for hver, med tegnet, oppgaven og tiden. |
| **orden** | `nerdr` deler lista i aktive og ledige. `mappe` har en gruppe for hvert repo, i alfabetisk rekkefølge, og Claude-ene i hver står der de er i tmux. `aktivitet` har ingen grupper, og den som sist endret tilstand, står øverst. |
| **ledig** | Om de ledige står rett `under` de aktive, eller begynner på `midten` av lista, så de ikke flytter seg når en blir aktiv. |
| **tittel** | Hvor mange linjer oppgaven får i boksen, fra `1` til `3`. |

Standard er farge på, vanlig plass, ordnet som nerdr med de ledige under, og
én linje til oppgaven. `ledig` gjelder bare nerdr, og `tittel` gjelder ikke
minimal. `nerdr liste` er alltid ordnet som nerdr.

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
