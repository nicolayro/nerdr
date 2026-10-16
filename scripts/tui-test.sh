#!/bin/sh
# Kjører nerdr inne i én tmux-server og lar den vise en annen, med et falskt
# sesjonsregister. Ingenting av det er brukerens eget. Bygg med `cargo build`
# først.
HER=$(cd "$(dirname "$0")" && pwd)
NERDR=$HER/../target/debug/nerdr
FALSK=$HER/falsk-claude.sh
KONFIG=$(mktemp -d)
mkdir "$KONFIG/sessions"
# HOME for nerdr, så mappevelgeren viser disse mappene og ikke brukerens egne.
mkdir -p "$KONFIG/git/prosjekt.ny" "$KONFIG/git/annet" "$KONFIG/fagdag/tredje"

tmux -L nerdr-maal kill-server 2>/dev/null
tmux -L nerdr-vert kill-server 2>/dev/null

tmux -L nerdr-maal -f /dev/null new-session -d -s alfa -x 80 -y 20 "sh $FALSK"
tmux -L nerdr-maal new-session -d -s beta -x 80 -y 20 /bin/sh
tmux -L nerdr-maal new-session -d -s gamma -x 80 -y 20 /bin/cat
# Nye sesjoner får cat i stedet for et skall, så «claude» som skrives inn,
# bare blir stående.
tmux -L nerdr-maal set -g default-command /bin/cat

# alfa venter og gamma jobber. beta er et skall uten Claude og skal ikke vises.
for linje in "alfa waiting" "gamma busy"; do
  set -- $linje
  pane=$(tmux -L nerdr-maal display-message -p -t "$1" '#{pane_id}')
  printf '{"tmux":"%s:@0.%s","status":"%s","cwd":"%s","updatedAt":%s000}' "$1" "$pane" "$2" "$(dirname "$HER")" "$(date +%s)" > "$KONFIG/sessions/$1.json"
done

tmux -L nerdr-vert -f /dev/null new-session -d -s vert -x 110 -y 16 \
  "HOME=$KONFIG NERDR_MAPPER=~/git:$KONFIG/fagdag CLAUDE_CONFIG_DIR=$KONFIG $NERDR -L nerdr-maal; sleep 30"
sleep 2

echo "=== nerdr etter oppstart (alfa ● under VENTER PÅ DEG, gamma ◐ under JOBBER, ingen beta) ==="
echo "=== kortene viser mappe › branch, og alfa har oppgaven under ==="
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

echo "=== alfa blir ferdig mens du er inne, og står fortsatt under VENTER PÅ DEG, med ○ ==="
alfa=$(tmux -L nerdr-maal display-message -p -t alfa '#{pane_id}')
for status in idle waiting; do
  printf '{"tmux":"alfa:@0.%s","status":"%s","cwd":"%s","updatedAt":%s000}' "$alfa" "$status" "$(dirname "$HER")" "$(date +%s)" > "$KONFIG/sessions/alfa.json"
  sleep 2.5
  [ $status = idle ] && tmux -L nerdr-vert capture-pane -p -t vert | head -3
done

echo "=== C-q tilbake, j ned til gamma ==="
tmux -L nerdr-vert send-keys -t vert C-q
sleep 0.2
tmux -L nerdr-vert send-keys -t vert j
sleep 1
tmux -L nerdr-vert capture-pane -p -t vert | head -7

echo "=== klikk på andre rad går rett inn i alfa ==="
tmux -L nerdr-vert send-keys -t vert -l "$(printf '\033[<0;5;2M\033[<0;5;2m')"
sleep 1
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1,7p;16p'
tmux -L nerdr-vert send-keys -t vert C-q

echo "=== gamma blir ferdig og havner under VENTER PÅ DEG med ◆, og uten cwd står gamma:0 ==="
pane=$(tmux -L nerdr-maal display-message -p -t gamma '#{pane_id}')
printf '{"tmux":"gamma:@0.%s","status":"idle","updatedAt":%s000}' "$pane" "$(date +%s)" > "$KONFIG/sessions/gamma.json"
sleep 2.5
tmux -L nerdr-vert capture-pane -p -t vert | head -7

echo "=== inn i gamma og ut igjen, så står den under SETT med ○, nederst ==="
tmux -L nerdr-vert send-keys -t vert j Enter
sleep 0.3
tmux -L nerdr-vert send-keys -t vert C-q
sleep 0.5
tmux -L nerdr-vert capture-pane -p -t vert

echo "=== k opp til alfa, så klikk på gamma nederst (rad 14) går inn i gamma ==="
tmux -L nerdr-vert send-keys -t vert k
sleep 0.3
tmux -L nerdr-vert send-keys -t vert -l "$(printf '\033[<0;5;14M\033[<0;5;14m')"
sleep 0.5
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '2p;14p;16p'
tmux -L nerdr-vert send-keys -t vert C-q
sleep 0.2

echo "=== ? viser hjelpen, og en tast lukker den ==="
tmux -L nerdr-vert send-keys -t vert ?
sleep 0.3
tmux -L nerdr-vert capture-pane -p -t vert
tmux -L nerdr-vert send-keys -t vert x
sleep 0.3
tmux -L nerdr-vert capture-pane -p -t vert | tail -1

echo "=== n åpner popupen med mappene i NERDR_MAPPER, ~/git og fagdag ==="
tmux -L nerdr-vert send-keys -t vert n
sleep 0.5
tmux -L nerdr-vert capture-pane -p -t vert

echo "=== «pny» sorterer prosjekt.ny øverst ==="
tmux -L nerdr-vert send-keys -t vert -l pny
sleep 0.3
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '2,4p'

echo "=== Enter lager sesjonen prosjekt-ny, skriver inn claude, og nerdr er inne i den med ◌ og «starter …» ==="
tmux -L nerdr-vert send-keys -t vert Enter
sleep 0.8
tmux -L nerdr-maal list-sessions -F '#{session_name} #{pane_current_path}' | grep prosjekt
tmux -L nerdr-maal capture-pane -p -t prosjekt-ny | grep -c claude
tmux -L nerdr-vert capture-pane -p -t vert

echo "=== når den står i registeret, får den vanlig merke og mister «starter …» ==="
pane=$(tmux -L nerdr-maal display-message -p -t prosjekt-ny '#{pane_id}')
printf '{"tmux":"prosjekt-ny:@0.%s","status":"idle","cwd":"%s","updatedAt":%s000}' "$pane" "$KONFIG/git/prosjekt.ny" "$(date +%s)" > "$KONFIG/sessions/ny.json"
sleep 2.5
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '5,8p;16p'

echo "=== samme mappe igjen gir et nytt vindu i prosjekt-ny (skal være 2) ==="
tmux -L nerdr-vert send-keys -t vert C-q
sleep 0.2
tmux -L nerdr-vert send-keys -t vert n
sleep 0.3
tmux -L nerdr-vert send-keys -t vert -l pny
sleep 0.3
tmux -L nerdr-vert send-keys -t vert Enter
sleep 0.8
tmux -L nerdr-maal list-windows -t prosjekt-ny | wc -l

echo "=== Esc i popupen går tilbake til lista uten ny sesjon (skal være 4 sesjoner) ==="
tmux -L nerdr-vert send-keys -t vert C-q
sleep 0.2
tmux -L nerdr-vert send-keys -t vert n
sleep 0.3
tmux -L nerdr-vert send-keys -t vert Escape
sleep 0.5
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '4p;16p'
tmux -L nerdr-maal list-sessions | wc -l

echo "=== q avslutter ==="
tmux -L nerdr-vert send-keys -t vert q
sleep 0.5
tmux -L nerdr-maal list-clients

tmux -L nerdr-maal kill-server

echo "=== uten tmux-server lager nerdr en sesjon (skal være 1) ==="
tmux -L nerdr-tom kill-server 2>/dev/null
tmux -L nerdr-vert new-session -d -s tom "CLAUDE_CONFIG_DIR=$KONFIG $NERDR -L nerdr-tom; sleep 30"
sleep 1
tmux -L nerdr-tom list-sessions | wc -l
tmux -L nerdr-tom kill-server

echo "=== uten tmux sier nerdr fra ==="
PATH=/finnes-ikke "$NERDR"

tmux -L nerdr-vert kill-server
rm -r "$KONFIG"
