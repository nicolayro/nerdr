# nerdr

TUI og kommandoer for Claude-sesjoner i tmux. nerdr kjører ikke Claude selv.
tmux eier terminalene, og nerdr kobler seg til som en klient i control mode.

## Oppbygning

- `kjerne/` vet ikke hvordan noe vises: control mode (`kontroll`), enkeltstående
  tmux-kommandoer (`tmux`), panene (`paner`), hvordan et vindu er delt
  (`oppsett`), registeret (`register`), hva som venter på deg (`merke`),
  tidligere samtaler (`samtaler`), repoet og branchen (`git`), farger fra
  `capture-pane -e` (`sgr`), prefiks- og root-tabellen fra `list-keys` (`bindinger`)
  og å starte en Claude (`start`).
- `tui/` er binæren. `main.rs` har argumentene, hendelsene og svartypene,
  `app.rs` `App` med hendelsesløkka, og `app/` resten av metodene på `App`
  etter hva som utløser dem: `svar.rs` svarene fra tmux, `vindu.rs` vinduet og
  størrelsen, `tast.rs` tastene og kopimodus, `mus.rs` musen, `liste.rs`
  rekkefølgen i lista og `handling.rs` handlingene og nye Claude-er.
  `agent.rs` er hva et kort i agentlista er, `kommandoer.rs`
  `liste`, `samtaler` og `ny`, `ny.rs` mappevelgeren, `handlinger.rs`
  handlingene bak `:`, `tmuxtaster.rs` hva tastene fra tmux-oppsettet gjør i
  nerdr, `utseende.rs` valgene i innstillingene, og `palett.rs`
  fargene til terminalen, som markeringen og tonene blandes av.
- `tegn.rs` er tegningen, delt etter flate: `tegn/liste.rs` agentlista,
  `tegn/kort.rs` kortene og overskriftene, `tegn/faner.rs` fanene,
  `tegn/knapper.rs` knappene og hintet, `tegn/pane.rs` panene og rammene,
  `tegn/popup.rs` popupene og søkemenyene, `tegn/stil.rs` fargene og stilene,
  og `tegn/treff.rs` hva musen står over (`Mål`).

## Begreper

- **Sesjon**, **vindu** og **pane** er alltid tmux sine.
- **Claude** er en Claude Code som kjører i en pane (`register::Claude`), og
  **samtale** er den Claude Code kaller `sessionId`.
- **Tilstand** er det Claude melder selv: spør, jobber eller klar.
- **Agent** er et kort i agentlista: en Claude, en nerdr nettopp har startet,
  eller et program som står igjen i en sesjon der Claude-en er borte.
- **Merke** er det kortet viser, og **gruppe** gir fargen og står i JSON.
  Det som venter på deg, er en Claude som spør, eller som er ferdig med noe
  du ikke har sett. Lista er delt i **aktive** og **ledige** (`grupper::Del`).

## Før commit

```
cargo test && cargo clippy --all-targets && cargo build && scripts/tui-test.sh
scripts/installer-test.sh    # når installer.sh eller avinstaller.sh er endret
```

`tui-test.sh` skriver bare ut sjekkene som feiler, med skjermen til nerdr.

## Konvensjoner

- Norsk overalt: navn, kommentarer, tekst i appen og commit-meldinger, med æ, ø
  og å.
- Kommentarer forklarer hvorfor, ikke hva.
- Få avhengigheter, ingen nettverkskall og ingen automatisk oppdatering.
- JSON fra kommandoene har `versjon`. Felt legges til, men endres ikke.

## Formatene til Claude Code

Ingen av dem er dokumentert. Felt som mangler eller har ukjente verdier, gir
ingen tilstand, aldri en feil.

- `~/.claude/sessions/<pid>.json`, eller under `$CLAUDE_CONFIG_DIR`: `tmux`
  (`sesjon:@vindu.%pane`), `status` (`waiting`, `busy`, `idle`),
  `statusUpdatedAt`, `cwd` og `sessionId`.
- `~/.claude/projects/<mappe>/<id>.jsonl`: `custom-title`, `ai-title`,
  `last-prompt` og `cwd` skrives på nytt nær slutten etter hver tur, så bare de
  siste 256 KB leses.
- Panetittelen er oppgaven, etter tegnet foran (`✳ Fiks testene`).
