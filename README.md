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
| `q` | Avslutt, fra lista |

Foran hver Claude står `●` når den venter på deg, `◐` når den jobber og `○` når
den er klar.
