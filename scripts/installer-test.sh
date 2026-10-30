#!/bin/sh
# Kjører installer.sh og avinstaller.sh mot en release i en lokal mappe og et
# falskt HOME, så verken en ekte release eller brukerens mapper røres. Bygg med
# `cargo build` først.
NERDR=$(cd "$(dirname "$0")/.." && pwd)
T=$(mktemp -d)

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) maal=aarch64-apple-darwin ;;
  Darwin-x86_64) maal=x86_64-apple-darwin ;;
  *) maal=x86_64-unknown-linux-gnu ;;
esac
pakke=nerdr-$maal.tar.gz
for mappe in latest/download download/v0.1.0; do
  mkdir -p "$T/releases/$mappe"
  tar -czf "$T/releases/$mappe/$pakke" -C "$NERDR/target/debug" nerdr
  (cd "$T/releases/$mappe" && shasum -a 256 "$pakke" > "$pakke.sha256")
done

# Variabler gis som argumenter, siden `VAR=x funksjon` blir stående i sh.
installer() {
  env HOME="$T/hjem" NERDR_URL="file://$T/releases" "$@" sh "$NERDR/scripts/installer.sh" 2>&1 |
    grep -v 'står ikke i PATH' | sed "s|$T/hjem|~|"
}
avinstaller() {
  env HOME="$T/hjem" sh "$NERDR/scripts/avinstaller.sh" | sed "s|$T/hjem|~|"
}
nytt_hjem() {
  rm -rf "$T/hjem"
  for mappe in "$@"; do mkdir -p "$T/hjem/$mappe"; done
}

echo "=== med ~/opt/bin og ~/bin havner nerdr i ~/opt/bin ==="
nytt_hjem opt/bin bin
installer

echo "=== med bare ~/bin havner den i ~/bin ==="
nytt_hjem bin
installer

echo "=== uten noen av dem lages ~/.local/bin ==="
nytt_hjem
installer

echo "=== ligger den i ~/.local/bin, oppdateres den der selv om ~/bin finnes ==="
mkdir -p "$T/hjem/bin"
installer

echo "=== en lenke i ~/opt/bin røres ikke (exit=1) ==="
nytt_hjem opt/bin
ln -s /et/annet/sted "$T/hjem/opt/bin/nerdr"
installer
echo "exit=$(env HOME="$T/hjem" NERDR_URL="file://$T/releases" sh "$NERDR/scripts/installer.sh" >/dev/null 2>&1; echo $?)"

echo "=== NERDR_VERSJON=v0.1.0 henter den versjonen, v9 finnes ikke ==="
nytt_hjem
installer NERDR_VERSJON=v0.1.0
installer NERDR_VERSJON=v9 | grep -v '^curl:'

echo "=== feil sjekksum installerer ingenting ==="
nytt_hjem
printf '%064d  %s\n' 0 "$pakke" > "$T/releases/latest/download/$pakke.sha256"
installer
ls "$T/hjem/.local/bin" 2>/dev/null

tar -czf "$T/releases/latest/download/$pakke" -C "$NERDR/target/debug" nerdr
(cd "$T/releases/latest/download" && shasum -a 256 "$pakke" > "$pakke.sha256")
# Uten tmux i PATH, og med en brew som bare sier hva den ble bedt om.
mkdir -p "$T/brew"
printf '#!/bin/sh\necho "brew $*"\n' > "$T/brew/brew"
chmod +x "$T/brew/brew"
# Og en dnf med en sudo som bare kjører den, så ingenting spør etter passord.
mkdir -p "$T/dnf"
printf '#!/bin/sh\necho "dnf $*"\n' > "$T/dnf/dnf"
printf '#!/bin/sh\n"$@"\n' > "$T/dnf/sudo"
chmod +x "$T/dnf/dnf" "$T/dnf/sudo"
if env PATH=/usr/bin:/bin sh -c 'command -v tmux' >/dev/null; then
  echo "=== tmux ligger i /usr/bin, så tilfellene uten tmux hoppes over ==="
else
  echo "=== NERDR_TMUX=ja installerer tmux med brew ==="
  nytt_hjem
  installer PATH="$T/brew:/usr/bin:/bin" NERDR_TMUX=ja

  echo "=== NERDR_TMUX=nei skriver bare kommandoen ==="
  nytt_hjem
  installer PATH="$T/brew:/usr/bin:/bin" NERDR_TMUX=nei

  echo "=== uten brew brukes dnf ==="
  nytt_hjem
  installer PATH="$T/dnf:/usr/bin:/bin" NERDR_TMUX=ja

  echo "=== uten pakkebehandler sier den fra ==="
  nytt_hjem
  installer PATH=/usr/bin:/bin
fi

echo "=== avinstaller fjerner filen og lar lenken stå ==="
nytt_hjem opt/bin bin
ln -s /et/annet/sted "$T/hjem/opt/bin/nerdr"
cp "$NERDR/target/debug/nerdr" "$T/hjem/bin/nerdr"
avinstaller
ls -A "$T/hjem/bin"

echo "=== uten nerdr sier avinstaller fra ==="
nytt_hjem
avinstaller

rm -rf "$T"
