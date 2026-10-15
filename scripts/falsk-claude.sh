#!/bin/sh
# Later som Claude Code: setter tittelen med ✳, skriver farger og en åpen dialog,
# og ekkoer det som skrives inn.
printf '\033]2;✳ Fiks testene\007'
printf '\033[38;2;217;119;87m● Claude\033[0m skriver \033[1mfet\033[0m og \033[48;5;22m grønn bakgrunn \033[0m\n'
printf 'Do you want to proceed?\n❯ 1. Yes\n  2. No\n\nEsc to cancel\n'
exec cat
