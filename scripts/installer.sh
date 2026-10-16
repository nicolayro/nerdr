#!/bin/sh
# Installerer nerdr fra siste release i ~/.local/bin, eller i NERDR_BIN. Kjør
# på nytt for å oppdatere.
#
# Repoet er privat, så nedlastingen går gjennom gh, som må være logget inn med
# tilgang til det.
set -eu

REPO=nicolayro/nerdr
BIN=${NERDR_BIN:-$HOME/.local/bin}

bygg_selv() {
  echo "Bygg den selv med Rust 1.85 eller nyere:" >&2
  echo "CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install --locked --git https://github.com/$REPO nerdr" >&2
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

if ! command -v gh >/dev/null; then
  echo "nerdr: installeringen trenger gh (GitHub CLI), logget inn med tilgang til $REPO." >&2
  exit 1
fi

if ! feil=$(gh release view --repo "$REPO" 2>&1 >/dev/null); then
  case $feil in
    *"not found"*)
      echo "nerdr: $REPO har ingen release ennå." >&2
      bygg_selv
      ;;
    *) echo "nerdr: $feil" >&2; exit 1 ;;
  esac
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
pakke=nerdr-$maal.tar.gz
gh release download --repo "$REPO" --pattern "$pakke*" --dir "$tmp"

if command -v shasum >/dev/null; then
  (cd "$tmp" && shasum -a 256 -c "$pakke.sha256" >/dev/null)
else
  (cd "$tmp" && sha256sum -c "$pakke.sha256" >/dev/null)
fi

mkdir -p "$BIN"
tar -xzf "$tmp/$pakke" -C "$BIN"
echo "$("$BIN/nerdr" --version) er installert i $BIN/nerdr."

case ":$PATH:" in
  *":$BIN:"*) ;;
  *) echo "$BIN står ikke i PATH. Legg til i skallets oppstartsfil: export PATH=\"$BIN:\$PATH\"" ;;
esac
command -v tmux >/dev/null || echo "nerdr trenger tmux. Installer den, for eksempel med: brew install tmux"
