#!/bin/sh
# Installerer nerdr fra en release på GitHub. Kjør på nytt for å oppdatere.
#
#   NERDR_VERSJON  en bestemt versjon, som v0.1.0, ellers den siste
#   NERDR_BIN      mappa nerdr legges i, ellers der den ligger fra før, eller
#                  den første som finnes av ~/opt/bin, ~/bin og ~/.local/bin
set -eu

REPO=nicolayro/nerdr
# Kan pekes et annet sted, som et speil eller en mappe i en test.
URL=${NERDR_URL:-https://github.com/$REPO/releases}

bygg_selv() {
  echo "Bygg den selv med Rust 1.85 eller nyere:" >&2
  echo "  cargo install --locked --git https://github.com/$REPO nerdr" >&2
  exit 1
}

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) maal=aarch64-apple-darwin ;;
  Darwin-x86_64) maal=x86_64-apple-darwin ;;
  Linux-x86_64) maal=x86_64-unknown-linux-gnu ;;
  *)
    echo "nerdr: det finnes ingen ferdig binær for $(uname -s) $(uname -m)." >&2
    bygg_selv
    ;;
esac

bin=
if [ -n "${NERDR_BIN:-}" ]; then
  bin=$NERDR_BIN
else
  # En oppdatering skal erstatte den som er der, ikke legge en til ved siden av.
  for mappe in "$HOME/opt/bin" "$HOME/bin" "$HOME/.local/bin"; do
    if [ -f "$mappe/nerdr" ] && [ ! -L "$mappe/nerdr" ]; then bin=$mappe; break; fi
  done
  if [ -z "$bin" ]; then
    for mappe in "$HOME/opt/bin" "$HOME/bin" "$HOME/.local/bin"; do
      bin=$mappe
      [ -d "$mappe" ] && break
    done
  fi
fi

if [ -L "$bin/nerdr" ]; then
  echo "nerdr: $bin/nerdr er en lenke til $(readlink "$bin/nerdr"), så nerdr er installert på en annen måte." >&2
  echo "Sett NERDR_BIN for å installere et annet sted." >&2
  exit 1
fi

if [ -n "${NERDR_VERSJON:-}" ]; then
  kilde=$URL/download/$NERDR_VERSJON
else
  kilde=$URL/latest/download
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
pakke=nerdr-$maal.tar.gz
for fil in "$pakke" "$pakke.sha256"; do
  if ! curl -fsSL -o "$tmp/$fil" "$kilde/$fil"; then
    echo "nerdr: fant ikke $fil i ${NERDR_VERSJON:-siste release}." >&2
    bygg_selv
  fi
done

if command -v shasum >/dev/null; then sjekk="shasum -a 256 -c"; else sjekk="sha256sum -c"; fi
if ! (cd "$tmp" && $sjekk "$pakke.sha256" >/dev/null 2>&1); then
  echo "nerdr: sjekksummen til $pakke stemmer ikke, så ingenting er installert." >&2
  exit 1
fi

# Pakket ut ved siden av og flyttet på plass, så en nerdr som kjører, ikke
# skrives over mens den er i bruk.
mkdir -p "$bin"
tar -xzf "$tmp/$pakke" -C "$tmp"
mv "$tmp/nerdr" "$bin/nerdr.ny"
mv "$bin/nerdr.ny" "$bin/nerdr"
echo "$("$bin/nerdr" --version) er installert i $bin/nerdr."

case ":$PATH:" in
  *":$bin:"*) ;;
  *) echo "$bin står ikke i PATH. Legg til i skallets oppstartsfil: export PATH=\"$bin:\$PATH\"" ;;
esac
command -v tmux >/dev/null || echo "nerdr trenger tmux. Installer den, for eksempel med: brew install tmux"
