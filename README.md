# nerdr

Et CLI som viser tmux-panene dine i en liste til venstre og strømmer den valgte
panen live til høyre, så du kan følge og skrive til Claude-sesjonene dine fra
ett sted.

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
| `↑` `↓` / `k` `j` | Velg pane |
| `⏎` | Gå inn i panen |
| `C-]` | Tilbake til lista |
| `q` | Avslutt, fra lista |
