# Notater

## Retning

nerdr skal gi oversikt over og navigasjon mellom Claude-sesjonene, og kjøre i
terminalen, slik herdr gjør. herdr er ikke brukt fordi et eksternt verktøy må
risikovurderes på nytt når kildekoden endres. Med et eget verktøy styres
endringene her, og det kan deles med kolleger. Det holdes lite: få
avhengigheter, ingen nettverkskall og ingen automatisk oppdatering.

tmux eier terminalene. nerdr er en klient som strømmer panene og tegner selv.
Terminalfunksjoner som mangler, som scrollback, kopiering og innliming, hentes
fra tmux (`copy-mode`, `capture-pane -M`, `paste-buffer -p`) heller enn å
lages på nytt.

### Åpen beslutning: terminal i samme område

nerdr viser bare den valgte Claude-en. Neste store steg er å kunne kjøre en
terminal i samme område, med vinduer og deling i paner. To veier:

- **tmux:** nerdr viser hele det valgte tmux-vinduet, med panene plassert slik
  `list-panes` sier (`pane_left`, `pane_top`, størrelse), og lar tmux dele og
  flytte (`split-window`, `select-pane`, `new-window`).
- **Eget:** nerdr holder selv rede på vinduer og deling, og bruker tmux bare
  til å eie hver terminal.

## Slik henger det sammen

nerdr kjører ikke Claude selv. tmux eier terminalene, og nerdr kobler seg til
som en klient i control mode (`tmux -C`), slik iTerm2 gjør.

- `kjerne/` vet ikke hvordan noe vises: control mode-klienten, panene fra
  `list-panes`, tilstanden fra sesjonsregisteret og tolkningen av farger fra
  `capture-pane -e`.
- `tui/` er ratatui-klienten.

Lista og tilstanden (venter, jobber, klar) kommer fra registeret Claude Code
selv skriver i `~/.claude/sessions/<pid>.json`, eller `$CLAUDE_CONFIG_DIR` når
den er satt. Bare paner som står der, vises. `status` er `waiting`, `busy` eller
`idle`, og `tmux` sier hvilken pane sesjonen kjører i (`sesjon:@vindu.%pane`).
Formatet er internt og ikke dokumentert, så ukjente verdier gir ingen tilstand.

Registeret skiller ikke mellom en Claude som er ferdig med noe du ikke har sett,
og en du har lest og latt ligge. Begge er `idle`. nerdr husker derfor når du sist
var inne i hver pane, og en `idle` med nyere `statusUpdatedAt` står under
«venter på deg». Det huskes ikke mellom kjøringer, så ved oppstart regnes alle
som sett.

Oppgaven er panetittelen Claude Code setter, etter tegnet foran (`✳ Fiks
testene`). En tittel uten et slikt tegn er satt av noe annet, som vertsnavnet,
og vises ikke. Branchen leses fra `.git/HEAD` i `cwd`, og gjennom `gitdir:` i
worktrees, så nerdr starter ikke git.

Skjermbildet hentes med `capture-pane -e` når tmux melder `%output` for panen
som vises, høyst 60 ganger i sekundet. Claude Code tømmer og tegner hele
skjermen i hver oppdatering, pakket i en synkronisert oppdatering
(`\033[?2026h` … `\033[?2026l`), så nerdr venter til den er ferdig før den
henter. nerdr pakker også sine egne tegninger på samme måte.

En ny Claude startes i en tmux-sesjon som heter det samme som mappa, med punktum
og kolon byttet ut med bindestrek, siden tmux ellers bytter dem ut selv og navnet
ikke finner sesjonen igjen. Vinduet starter med et vanlig skall, og `claude`
skrives inn med `send-keys`. Kjører tmux kommandoen direkte, går den gjennom
`zsh -c`, som ikke leser `.zshrc`, og da hopper den over en `claude`-funksjon der,
som wrapperen til teamet. Når Claude avslutter, står du i skallet i samme mappe.

Mappevelgeren er nerdr sin egen, ikke fzf. nerdr står utenfor tmux, så
`fzf --popup` virker ikke, og `fzf --height` tømmer hele radene den bruker og
alt under seg. Å gi terminalen til fzf betyr også at tastetråden må stå stille,
for `event::poll` i crossterm leser tastene inn i sin egen buffer.

Det tar flere sekunder før en ny Claude står i registeret. Panen vises derfor
fra tmux har laget den, med `◌` og «starter …», og nerdr går inn i den.

`-L <socket>` velger tmux-server, som `tmux -L`. `scripts/tui-test.sh` bruker
det, sammen med et falskt register i `CLAUDE_CONFIG_DIR`, til å kjøre nerdr mot
en falsk Claude på to egne tmux-servere.

## Kjente begrensninger

- nerdr velger vinduet den viser. Andre klienter i samme sesjon bytter vindu
  samtidig, og vinduet får størrelsen til nerdr.
- Scrolling skjer ved at musehjulet sendes til panen. Claude Code i fullskjerm
  ber om mus og scroller selv. En pane som ikke ber om mus, kan ikke scrolles,
  og tmux-historikken (`copy-mode`, `capture-pane -M`) er ikke tatt i bruk.
- nerdr tar musen, så for å markere tekst med terminalen selv må du holde
  Option (iTerm2) eller Shift (de fleste andre).
- Registeret sier ikke hvilken tmux-server en sesjon kjører på. Med `-L` kan en
  pane få tilstanden til panen med samme id på standardserveren.

## Avhengigheter

`ratatui` med `default-features = false` og `crossterm`, og `serde` med
`serde_json` for registeret: 69 tredjeparts-crates på macOS, 27 av dem fra
crossterm og 6 fra serde.

Alternativer å vurdere:

- **`vt100` og `tui-term`** i stedet for `capture-pane -e`: nerdr tolker
  bytestrømmen fra `%output` selv. Omtrent fem crates til.
- **crossterm uten ratatui**: en egen liten buffer-diff, rundt 150 linjer.
  Halverer antall crates.

## Senere iterasjoner

**Ny mappe.** `n` starter Claude i en mappe som finnes. Neste steg er å kunne
lage en ny, for eksempel et worktree, slik `ny-sesjon.sh` i sidebar-repoet gjør.

**Mappene per bruker.** Mappene det søkes i, er faste (`MAPPER` i `tui/src/ny.rs`).
Før nerdr deles med andre, må de kunne settes per bruker, for eksempel med en
miljøvariabel.

Branchen `full-prototype` har en første versjon av tilstand per Claude
(venter, jobber, klar) oversatt fra `sesjoner.py` i sidebar-repoet,
gruppering på repo og worktree, `--json`, mus og innliming.
