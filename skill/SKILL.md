---
name: nerdr
description: Les og styr de andre Claude-ene i tmux med nerdr. Bruk når brukeren viser til en annen sesjon eller Claude, som «spør den som jobber i backend», «hva holder de andre på med», «la en annen Claude reviewe dette» eller «start 10 sesjoner som …».
allowed-tools: Bash(nerdr:*), Bash(tmux send-keys:*), Bash(tmux capture-pane:*), Bash(printenv TMUX_PANE), Bash(echo ingen, du kjører utenfor tmux), Bash(sleep:*)
---

Du er pane !`printenv TMUX_PANE || echo ingen, du kjører utenfor tmux`. Claude-ene i tmux:

!`nerdr liste`

- **Lese:** `nerdr liste --json` gir `tilstand`, `mappe` og `samtale`.
  `tmux capture-pane -p -t <pane> -S -200` gir skjermen. Hele samtalen ligger
  i `~/.claude/projects/<mappe med / og . byttet til ->/<samtale>.jsonl`, og
  svaret står sist.
- **Sende:** `tmux send-keys -t <pane> -l "<tekst>"`, og så
  `tmux send-keys -t <pane> Enter` i et eget kall. Skriv på én linje, for et
  linjeskift sender meldingen. Står teksten igjen i feltet, send `Enter` igjen.
- **Starte:** `nerdr ny <mappe> --json` gir panen. Vent til den er `klar` før du
  sender oppgaven. `nerdr ny --fortsett <id>` fortsetter en samtale fra
  `nerdr samtaler`.
- **Vente på svar:** `sleep` og `nerdr liste --json` til `tilstand` er `klar`.
  Les så svaret.

Den andre Claude-en vet bare det du skriver til den, så ta med konteksten den
trenger, og be den avslutte med et sammendrag. Skal flere endre filer i samme
repo, be hver jobbe i et eget worktree.

Send ikke til en Claude som `spør`, for da blir teksten svaret på spørsmålet.
Skriv aldri til din egen pane, og spør brukeren før du lukker en pane.
