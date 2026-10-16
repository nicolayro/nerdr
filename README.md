# nerdr

Et CLI som viser Claude-sesjonene dine i tmux i en liste til venstre og strømmer
den valgte live til høyre, så du kan følge og skrive til dem fra ett sted.

## Forutsetninger

- Rust 1.85 eller nyere (edition 2024)
- tmux med minst én sesjon (testet med 3.8)

## Kom i gang

```
git clone https://github.com/nicolayro/nerdr
cd nerdr
cargo run --release
```

Kjør den i en vanlig terminalfane, utenfor tmux.

| Tast | Hva |
| --- | --- |
| `↑` `↓` / `k` `j` | Velg Claude |
| `⏎` | Gå inn i sesjonen |
| `C-]` | Tilbake til lista |
| `n` | Ny Claude i en mappe du velger |
| `q` | Avslutt, fra lista |

Du kan også bruke musen. Klikk i lista velger en Claude, og klikk og musehjul i
sesjonen går videre til Claude.

Lista er delt i tre. Under **venter på deg** står de som spør om noe (`●`) og de
som er ferdige med noe du ikke har sett (`◆`). Så kommer de som **jobber** (`◐`),
og til slutt de du har **sett** (`○`). En ferdig Claude regnes som sett når du
har vært inne i den.

Hver Claude viser mappen og branchen den jobber i, og under det oppgaven den
holder på med.

`n` åpner en popup der du søker fram en mappe under `~/git`, `~/worktree` eller
`~/fagdag`, og `⏎` starter `claude` der i en tmux-sesjon med samme navn som
mappa. Finnes sesjonen allerede, får den et nytt vindu. Du er inne i den med en
gang, og kortet står med `◌` til Claude er klar.
