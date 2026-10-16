#!/bin/sh
# Fjerner nerdr fra mappene installer.sh bruker. nerdr lagrer ingenting annet.
#
# Bare vanlige filer slettes. En lenke kan høre til en annen installasjon
# eller et eget bygg.
set -eu

funnet=
for mappe in ${NERDR_BIN:+"$NERDR_BIN"} "$HOME/opt/bin" "$HOME/bin" "$HOME/.local/bin"; do
  fil=$mappe/nerdr
  if [ -L "$fil" ]; then
    echo "$fil er en lenke til $(readlink "$fil"), og er ikke fjernet."
    funnet=1
  elif [ -f "$fil" ]; then
    rm "$fil"
    echo "Fjernet $fil."
    funnet=1
  fi
done

[ -n "$funnet" ] || echo "Fant ingen nerdr i ~/opt/bin, ~/bin eller ~/.local/bin."
if [ -e "$HOME/.cargo/bin/nerdr" ]; then
  echo "Det finnes også en nerdr i ~/.cargo/bin. Er den installert med cargo, fjernes den med: cargo uninstall nerdr"
fi
if [ -n "${NERDR_MAPPER:-}" ]; then
  echo "NERDR_MAPPER er satt. Linja kan fjernes fra skallets oppstartsfil."
fi
