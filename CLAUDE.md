# nerdr

TUI og kommandoer for Claude-sesjoner i tmux. nerdr kjører ikke Claude selv.
tmux eier terminalene, og nerdr kobler seg til som en klient i control mode.

## Oppbygning

- `kjerne/` vet ikke hvordan noe vises: control mode (`kontroll`), enkeltstående
  tmux-kommandoer (`tmux`), panene (`paner`), hvordan et vindu er delt
  (`oppsett`), registeret (`register`), hva som venter på deg (`merke`),
  tidligere samtaler (`samtaler`), repoet og branchen (`git`), farger fra
  `capture-pane -e` (`sgr`) og å starte en Claude (`start`).
- `tui/` er binæren. `main.rs` har TUI-et og argumentene, `kommandoer.rs`
  `liste`, `samtaler` og `ny`, `ny.rs` mappevelgeren, `meny.rs` «Ny»-menyen,
  `utseende.rs` valgene i utseende-menyen, `tegn.rs` tegningen, `tegn/kort.rs`
  kortene og overskriftene i lista, og `palett.rs` fargene til terminalen, som
  markeringen og tonene blandes av.

## Begreper

- **Sesjon**, **vindu** og **pane** er alltid tmux sine.
- **Claude** er en Claude Code som kjører i en pane (`register::Claude`), og
  **samtale** er den Claude Code kaller `sessionId`.
- **Tilstand** er det Claude melder selv: spør, jobber eller klar.
- **Merke** og **gruppe** er det lista viser. VENTER er det som venter på deg:
  en Claude som spør, eller som er ferdig med noe du ikke har sett.

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
