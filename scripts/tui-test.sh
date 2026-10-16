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
mkdir -p "$KONFIG/git/prosjekt.ny" "$KONFIG/git/annet" "$KONFIG/kode/tredje"
# En tidligere samtale i annet, og en tom en som ikke skal vises.
mkdir -p "$KONFIG/projects/annet"
printf '{"type":"user","cwd":"%s"}\n{"type":"ai-title","aiTitle":"Fiks testene"}\n' "$KONFIG/git/annet" > "$KONFIG/projects/annet/abc-123.jsonl"
printf '{"type":"user","cwd":"%s"}\n{"type":"last-prompt"}\n' "$KONFIG/git/annet" > "$KONFIG/projects/annet/tom.jsonl"

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
  "HOME=$KONFIG NERDR_MAPPER=~/git:$KONFIG/kode CLAUDE_CONFIG_DIR=$KONFIG $NERDR -L nerdr-maal; sleep 30"
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

echo "=== n åpner popupen med mappene i NERDR_MAPPER, ~/git og kode ==="
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

echo "=== C-r i lista viser tidligere samtaler: bare «annet  Fiks testene», 1/1 ==="
tmux -L nerdr-vert send-keys -t vert C-q
sleep 0.2
tmux -L nerdr-vert send-keys -t vert C-r
sleep 0.3
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1,4p'

echo "=== C-r der bytter til mappene og tilbake ==="
tmux -L nerdr-vert send-keys -t vert C-r
sleep 0.3
tmux -L nerdr-vert capture-pane -p -t vert | grep -o -e 'tidligere samtaler' -e 'ny claude' | head -1
tmux -L nerdr-vert send-keys -t vert C-r
sleep 0.3
tmux -L nerdr-vert capture-pane -p -t vert | grep -o -e 'tidligere samtaler' -e 'ny claude' | head -1

echo "=== Enter lager sesjonen annet og skriver inn claude --resume abc-123 ==="
tmux -L nerdr-vert send-keys -t vert Enter
sleep 0.8
tmux -L nerdr-maal capture-pane -p -t annet | grep -m1 'claude --resume'

echo "=== Esc i popupen går tilbake til lista uten ny sesjon (skal være 5 sesjoner) ==="
tmux -L nerdr-vert send-keys -t vert C-q
sleep 0.2
tmux -L nerdr-vert send-keys -t vert n
sleep 0.3
tmux -L nerdr-vert send-keys -t vert Escape
sleep 0.5
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '4p;16p'
tmux -L nerdr-maal list-sessions | wc -l

echo "=== t på alfa lager et vindu rett etter alfa:0, merket med alfa, og nerdr er inne i det ==="
tmux -L nerdr-vert send-keys -t vert k k k k k k k k
sleep 0.3
tmux -L nerdr-vert send-keys -t vert t
sleep 1
tmux -L nerdr-maal list-panes -s -t alfa -F '#{window_index} #{pane_current_command} #{@nerdr-claude}' | sed "s/$alfa/alfa/"
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1,4p;16p'

echo "=== det du skriver, går til terminalen (cat skriver «i terminalen» tilbake) ==="
tmux -L nerdr-vert send-keys -t vert -l 'i terminalen'
tmux -L nerdr-vert send-keys -t vert Enter
sleep 0.8
tmux -L nerdr-maal capture-pane -p -t alfa:1 | grep -c 'i terminalen'

echo "=== 30 linjer i terminalen, og bunnen viser linje 30 ==="
for i in $(seq 1 30); do tmux -L nerdr-maal send-keys -t alfa:1 "linje $i" Enter; done
sleep 0.8
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '15p'

echo "=== ti hakk opp er 30 linjer, og cat med ekko gir linje 8 og 9 øverst, med 30/47 oppe til høyre ==="
for i in $(seq 1 10); do tmux -L nerdr-vert send-keys -t vert -l "$(printf '\033[<64;41;4M')"; done
sleep 0.8
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1,2p'

echo "=== to hakk ned gir linje 11 og 24/47, og Enter går tilbake til bunnen uten teller, med to tomme linjer nederst ==="
for i in 1 2; do tmux -L nerdr-vert send-keys -t vert -l "$(printf '\033[<65;41;4M')"; done
sleep 0.5
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1p'
tmux -L nerdr-vert send-keys -t vert Enter
sleep 0.5
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1p;14,15p'

echo "=== C-q, k opp til alfa, og t går inn i samme terminal (skal være 2 vinduer) ==="
tmux -L nerdr-vert send-keys -t vert C-q
sleep 0.2
tmux -L nerdr-vert send-keys -t vert k
sleep 0.3
tmux -L nerdr-vert send-keys -t vert t
sleep 0.8
tmux -L nerdr-maal list-windows -t alfa | wc -l
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '16p'

echo "=== C-d avslutter cat, vinduet lukkes, og nerdr står i lista med alfa valgt ==="
tmux -L nerdr-vert send-keys -t vert C-d
sleep 1.5
tmux -L nerdr-maal list-windows -t alfa | wc -l
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1,4p;16p'

echo "=== alfa delt i to i tmux: Claude til venstre, │ i kolonne 37, cat til høyre ==="
tmux -L nerdr-maal split-window -h -t alfa:0
sleep 1
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1,3p'

echo "=== Enter går inn i Claude-en og ikke i den nye panen, så «til venstre» havner i alfa:0.0 ==="
tmux -L nerdr-vert send-keys -t vert Enter
sleep 0.3
tmux -L nerdr-vert send-keys -t vert -l 'til venstre'
sleep 0.5
tmux -L nerdr-maal capture-pane -p -t alfa:0.0 | grep -c 'til venstre'

echo "=== klikk i høyre pane gjør den aktiv, og «til høyre» havner i alfa:0.1 ==="
tmux -L nerdr-vert send-keys -t vert -l "$(printf '\033[<0;90;5M\033[<0;90;5m')"
sleep 0.3
tmux -L nerdr-vert send-keys -t vert -l 'til høyre'
sleep 0.5
tmux -L nerdr-maal capture-pane -p -t alfa:0.1 | grep -c 'til høyre'
tmux -L nerdr-maal display-message -p -t alfa:0 '#{pane_index}'
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1,2p'

echo "=== når høyre pane lukkes, fyller Claude-en hele visningen igjen (74x16) ==="
tmux -L nerdr-maal kill-pane -t alfa:0.1
sleep 1
tmux -L nerdr-maal display-message -p -t alfa:0.0 '#{pane_width}x#{pane_height}'
tmux -L nerdr-vert send-keys -t vert C-q

echo "=== q avslutter ==="
tmux -L nerdr-vert send-keys -t vert q
sleep 0.5
tmux -L nerdr-maal list-clients

echo "=== nerdr liste: alfa venter, gamma og prosjekt.ny er klare ==="
CLAUDE_CONFIG_DIR=$KONFIG "$NERDR" -L nerdr-maal liste
CLAUDE_CONFIG_DIR=$KONFIG "$NERDR" -L nerdr-maal liste --json | grep -o -e '"versjon":1' -e '"tilstand":"[a-z]*"' | tr '\n' ' '
echo

echo "=== nerdr samtaler: Fiks testene i annet, med id abc-123 ==="
CLAUDE_CONFIG_DIR=$KONFIG "$NERDR" samtaler

echo "=== nerdr ny lager sesjonen tredje og skriver inn claude ==="
CLAUDE_CONFIG_DIR=$KONFIG "$NERDR" -L nerdr-maal ny "$KONFIG/kode/tredje" | sed "s|$KONFIG|…|"
sleep 0.5
tmux -L nerdr-maal capture-pane -p -t tredje | grep -m1 claude

echo "=== nerdr ny --fortsett abc-123 finner mappa annet og gir et nytt vindu der (skal være 2) ==="
CLAUDE_CONFIG_DIR=$KONFIG "$NERDR" -L nerdr-maal ny --fortsett abc-123 --json | grep -o '"sesjon":"[a-z]*"'
sleep 0.5
tmux -L nerdr-maal list-windows -t annet | wc -l
tmux -L nerdr-maal capture-pane -p -t annet:1 | grep -m1 'claude --resume'

echo "=== en samtale som kjører, startes ikke en gang til, og ukjent id gir feil ==="
pane=$(tmux -L nerdr-maal display-message -p -t tredje '#{pane_id}')
printf '{"tmux":"tredje:@0.%s","status":"busy","sessionId":"abc-123","updatedAt":%s000}' "$pane" "$(date +%s)" > "$KONFIG/sessions/tredje.json"
CLAUDE_CONFIG_DIR=$KONFIG "$NERDR" -L nerdr-maal ny --fortsett abc-123
CLAUDE_CONFIG_DIR=$KONFIG "$NERDR" -L nerdr-maal ny --fortsett finnes-ikke

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
