#!/bin/sh
# Kjører nerdr inne i én tmux-server og lar den vise en annen, med et falskt
# sesjonsregister. Ingenting av det er brukerens eget. Bygg med `cargo build`
# først.
HER=$(cd "$(dirname "$0")" && pwd)
NERDR=$HER/../target/debug/nerdr
FALSK=$HER/falsk-claude.sh
KONFIG=$(mktemp -d)
mkdir "$KONFIG/sessions"

tmux -L nerdr-maal kill-server 2>/dev/null
tmux -L nerdr-vert kill-server 2>/dev/null

tmux -L nerdr-maal -f /dev/null new-session -d -s alfa -x 80 -y 20 "sh $FALSK"
tmux -L nerdr-maal new-session -d -s beta -x 80 -y 20 /bin/sh
tmux -L nerdr-maal new-session -d -s gamma -x 80 -y 20 /bin/cat

# alfa venter og gamma jobber. beta er et skall uten Claude og skal ikke vises.
for linje in "alfa waiting" "gamma busy"; do
  set -- $linje
  pane=$(tmux -L nerdr-maal display-message -p -t "$1" '#{pane_id}')
  printf '{"tmux":"%s:@0.%s","status":"%s","updatedAt":1}' "$1" "$pane" "$2" > "$KONFIG/sessions/$1.json"
done

tmux -L nerdr-vert -f /dev/null new-session -d -s vert -x 110 -y 16 \
  "CLAUDE_CONFIG_DIR=$KONFIG $NERDR -L nerdr-maal; sleep 30"
sleep 2

echo "=== nerdr etter oppstart (alfa ● og gamma ◐, ingen beta) ==="
tmux -L nerdr-vert capture-pane -p -t vert
echo "=== størrelse på alfa (skal være 74x16) ==="
tmux -L nerdr-maal display-message -p -t alfa '#{window_width}x#{window_height}'

echo "=== tastetrykk: Enter for fokus, så «hei æ» og Enter til alfa ==="
tmux -L nerdr-vert send-keys -t vert Enter
sleep 0.3
tmux -L nerdr-vert send-keys -t vert -l 'hei æ'
tmux -L nerdr-vert send-keys -t vert Enter
sleep 0.8
tmux -L nerdr-maal capture-pane -p -t alfa | grep -n 'hei'

echo "=== innliming av to linjer, pakket i ^[[200~ … ^[[201~ ==="
tmux -L nerdr-vert set-buffer -b test "$(printf 'linje én\n$HOME "to"')"
tmux -L nerdr-vert paste-buffer -p -d -b test -t vert
sleep 0.8
tmux -L nerdr-maal capture-pane -p -t alfa | grep -n -e 'linje' -e 'HOME'

echo "=== musehjul i kolonne 41, rad 4 (skal komme fram som ^[[<64;5;4M) ==="
tmux -L nerdr-vert send-keys -t vert -l "$(printf '\033[<64;41;4M')"
sleep 0.8
tmux -L nerdr-maal capture-pane -p -t alfa | grep -n '<64'

echo "=== C-] tilbake, j ned til gamma ==="
tmux -L nerdr-vert send-keys -t vert C-]
sleep 0.2
tmux -L nerdr-vert send-keys -t vert j
sleep 1
tmux -L nerdr-vert capture-pane -p -t vert | head -3

echo "=== klikk på første rad velger alfa igjen ==="
tmux -L nerdr-vert send-keys -t vert -l "$(printf '\033[<0;5;1M\033[<0;5;1m')"
sleep 1
tmux -L nerdr-vert capture-pane -p -t vert | head -3

echo "=== q avslutter ==="
tmux -L nerdr-vert send-keys -t vert q
sleep 0.5
tmux -L nerdr-maal list-clients

tmux -L nerdr-maal kill-server
tmux -L nerdr-vert kill-server
rm -r "$KONFIG"
