#!/bin/sh
# Kjører nerdr inne i én tmux-server og lar den vise en annen. Ingen av dem er
# brukerens egen server. Bygg med `cargo build` først.
HER=$(cd "$(dirname "$0")" && pwd)
NERDR=$HER/../target/debug/nerdr
FALSK=$HER/falsk-claude.sh

tmux -L nerdr-maal kill-server 2>/dev/null
tmux -L nerdr-vert kill-server 2>/dev/null

tmux -L nerdr-maal -f /dev/null new-session -d -s alfa -x 80 -y 20 "sh $FALSK"
tmux -L nerdr-maal new-session -d -s beta -x 80 -y 20 /bin/sh
tmux -L nerdr-vert -f /dev/null new-session -d -s vert -x 110 -y 16 "$NERDR -L nerdr-maal; sleep 30"
sleep 2

echo "=== nerdr etter oppstart ==="
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
tmux -L nerdr-vert capture-pane -p -t vert | tail -8

echo "=== C-] tilbake, j ned til beta ==="
tmux -L nerdr-vert send-keys -t vert C-]
sleep 0.2
tmux -L nerdr-vert send-keys -t vert j
sleep 1
tmux -L nerdr-vert capture-pane -p -t vert

echo "=== q avslutter ==="
tmux -L nerdr-vert send-keys -t vert q
sleep 0.5
tmux -L nerdr-vert capture-pane -p -t vert | head -3
tmux -L nerdr-maal list-clients

tmux -L nerdr-maal kill-server
tmux -L nerdr-vert kill-server
