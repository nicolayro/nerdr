#!/bin/sh
# Kjører nerdr inne i én tmux-server og lar den vise en annen, med et falskt
# sesjonsregister. Ingenting av det er brukerens eget. Bygg med `cargo build`
# først. Forventningene gjelder bokser, og NERDR_RAMMER=linjer foran kjører
# den med linjer.
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
  "HOME=$KONFIG NERDR_MAPPER=~/git:$KONFIG/kode NERDR_RAMMER=${NERDR_RAMMER:-bokser} CLAUDE_CONFIG_DIR=$KONFIG $NERDR -L nerdr-maal; sleep 30"
sleep 2

echo "=== nerdr etter oppstart (alfa ● under VENTER PÅ DEG, gamma ◐ under JOBBER, ingen beta) ==="
echo "=== kortene viser mappa med branchen helt til høyre, og alfa har oppgaven under. Øverst står fanen gcat på en strek som fortsetter overskriften med ┬, og de andre overskriftene treffer skillestreken med ┤. SETT står med 0 ==="
tmux -L nerdr-vert capture-pane -p -t vert
echo "=== størrelse på alfa (skal være 68x12 med bokser, 74x15 med linjer) ==="
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

echo "=== musehjul i kolonne 41, rad 4 (skal komme fram som ^[[<64;2;1M med bokser, ^[[<64;5;3M med linjer) ==="
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

echo "=== C-b w tilbake, j ned til gamma ==="
tmux -L nerdr-vert send-keys -t vert C-b w
sleep 0.2
tmux -L nerdr-vert send-keys -t vert j
sleep 1
tmux -L nerdr-vert capture-pane -p -t vert | head -7

echo "=== klikk på andre rad går rett inn i alfa ==="
tmux -L nerdr-vert send-keys -t vert -l "$(printf '\033[<0;5;2M\033[<0;5;2m')"
sleep 1
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1,7p;16p'
tmux -L nerdr-vert send-keys -t vert C-b w

echo "=== gamma blir ferdig og havner under VENTER PÅ DEG med ◆, og uten cwd står gamma:0 ==="
pane=$(tmux -L nerdr-maal display-message -p -t gamma '#{pane_id}')
printf '{"tmux":"gamma:@0.%s","status":"idle","updatedAt":%s000}' "$pane" "$(date +%s)" > "$KONFIG/sessions/gamma.json"
sleep 2.5
tmux -L nerdr-vert capture-pane -p -t vert | head -7

echo "=== inn i gamma og ut igjen, så står den under SETT med ○, nederst ==="
tmux -L nerdr-vert send-keys -t vert j Enter
sleep 0.3
tmux -L nerdr-vert send-keys -t vert C-b w
sleep 0.5
tmux -L nerdr-vert capture-pane -p -t vert

echo "=== k opp til alfa, så klikk på gamma nederst (rad 14) går inn i gamma ==="
tmux -L nerdr-vert send-keys -t vert k
sleep 0.3
tmux -L nerdr-vert send-keys -t vert -l "$(printf '\033[<0;5;14M\033[<0;5;14m')"
sleep 0.5
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '2p;14p;16p'
tmux -L nerdr-vert send-keys -t vert C-b w
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
tmux -L nerdr-vert send-keys -t vert C-b w
sleep 0.2
tmux -L nerdr-vert send-keys -t vert n
sleep 0.3
tmux -L nerdr-vert send-keys -t vert -l pny
sleep 0.3
tmux -L nerdr-vert send-keys -t vert Enter
sleep 0.8
tmux -L nerdr-maal list-windows -t prosjekt-ny | wc -l

echo "=== C-r i lista viser tidligere samtaler: bare «annet  Fiks testene», 1/1 ==="
tmux -L nerdr-vert send-keys -t vert C-b w
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
tmux -L nerdr-vert send-keys -t vert C-b w
sleep 0.2
tmux -L nerdr-vert send-keys -t vert n
sleep 0.3
tmux -L nerdr-vert send-keys -t vert Escape
sleep 0.5
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '4p;16p'
tmux -L nerdr-maal list-sessions | wc -l

echo "=== k opp til alfa ==="
tmux -L nerdr-vert send-keys -t vert k k k k k k k k
sleep 0.3

echo "=== C-b viser prefikstastene i hintlinja ==="
tmux -L nerdr-vert send-keys -t vert C-b
sleep 0.3
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '16p'

echo "=== % deler alfa i to bokser med luft mellom, begge i nerdr-mappa, og tmux får 64x12 (74x15 med linjer) ==="
tmux -L nerdr-vert send-keys -t vert %
sleep 1
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1,3p'
tmux -L nerdr-maal list-panes -t alfa:0 -F '#{pane_index} #{pane_current_path}' | sed "s|$(dirname "$HER")|nerdr|"
tmux -L nerdr-maal display-message -p -t alfa:0 '#{window_width}x#{window_height}'

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

echo "=== 30 linjer i høyre pane, og bunnen av boksen viser linje 30 ==="
for i in $(seq 1 30); do tmux -L nerdr-maal send-keys -t alfa:0.1 "linje $i" Enter; done
sleep 0.8
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '15p'

echo "=== fire hakk opp med hjulet over høyre pane gir 12/… på toppen av rammen ==="
for i in 1 2 3 4; do tmux -L nerdr-vert send-keys -t vert -l "$(printf '\033[<64;90;6M')"; done
sleep 0.8
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1,2p'

echo "=== to hakk ned gir 6/…, og Enter går tilbake til bunnen uten teller ==="
for i in 1 2; do tmux -L nerdr-vert send-keys -t vert -l "$(printf '\033[<65;90;6M')"; done
sleep 0.5
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1p'
tmux -L nerdr-vert send-keys -t vert Enter
sleep 0.5
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1p'

echo "=== C-b C-b sender ^B til cat, og C-b o gjør Claude-en aktiv igjen (0) ==="
tmux -L nerdr-vert send-keys -t vert C-b C-b
sleep 0.5
tmux -L nerdr-maal capture-pane -p -t alfa:0.1 | grep -c '\^B'
tmux -L nerdr-vert send-keys -t vert C-b o
sleep 0.5
tmux -L nerdr-maal display-message -p -t alfa:0 '#{pane_index}'

echo "=== C-b z viser bare Claude-en, og C-b z igjen gir begge panene tilbake ==="
tmux -L nerdr-vert send-keys -t vert C-b z
sleep 0.8
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1,2p'
tmux -L nerdr-vert send-keys -t vert C-b z
sleep 0.8
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1p'

echo "=== C-b ) og C-b ( bytter fra den delte alfa til neste Claude og til en annen igjen uten at noe vindu endrer størrelse (ingen linjer) ==="
tmux -L nerdr-maal set-hook -g window-resized "run-shell 'echo #{session_name} #{window_width}x#{window_height} >> $KONFIG/storrelser'"
tmux -L nerdr-vert send-keys -t vert C-b ')'
sleep 0.8
tmux -L nerdr-vert send-keys -t vert C-b '('
sleep 0.8
cat "$KONFIG/storrelser" 2>/dev/null
tmux -L nerdr-maal set-hook -gu window-resized
tmux -L nerdr-vert send-keys -t vert C-b w k k k k k k k k Enter
sleep 0.5

echo "=== C-b o, C-b x spør, n lar panen stå, og y lukker den (2, så 1) ==="
tmux -L nerdr-vert send-keys -t vert C-b o C-b x
sleep 0.3
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '16p'
tmux -L nerdr-vert send-keys -t vert n
sleep 0.3
tmux -L nerdr-maal list-panes -t alfa:0 | wc -l
tmux -L nerdr-vert send-keys -t vert C-b x y
sleep 1
tmux -L nerdr-maal list-panes -t alfa:0 | wc -l

echo "=== C-b \" deler over og under, med boksene rett under hverandre, og C-b x y lukker den nye ==="
tmux -L nerdr-vert send-keys -t vert C-b '"'
sleep 1
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '6,10p'
tmux -L nerdr-vert send-keys -t vert C-b x y
sleep 1

echo "=== uten split fyller Claude-en hele visningen igjen (68x12, 74x15 med linjer) ==="
tmux -L nerdr-maal display-message -p -t alfa:0.0 '#{pane_width}x#{pane_height}'
tmux -L nerdr-vert send-keys -t vert C-b w

echo "=== C-b B bytter rammene: alfa går fra 68x12 med bokser til 74x15 med linjer, og tilbake ==="
for i in 1 2; do
  tmux -L nerdr-vert send-keys -t vert C-b B
  sleep 0.8
  tmux -L nerdr-maal display-message -p -t alfa '#{window_width}x#{window_height}'
  tmux -L nerdr-vert capture-pane -p -t vert | sed -n '2p'
done

echo "=== C-b c fra lista lager alfa:1, nerdr er inne i det, og fanene gcat og tmux står øverst (navnet kan være tmux til tmux har gitt det navnet cat) ==="
tmux -L nerdr-vert send-keys -t vert C-b c
sleep 1
tmux -L nerdr-maal list-windows -t alfa -F '#{window_index} #{window_name}'
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1,2p;16p'

echo "=== det du skriver, havner i alfa:1 ==="
tmux -L nerdr-vert send-keys -t vert -l 'i vindu 1'
sleep 0.5
tmux -L nerdr-maal capture-pane -p -t alfa:1 | grep -c 'i vindu 1'

echo "=== C-b n går rundt til 0, C-b l tilbake til 1, og C-b 0 til 0 (0, 1, 0) ==="
for tast in n l 0; do
  tmux -L nerdr-vert send-keys -t vert C-b $tast
  sleep 0.5
  tmux -L nerdr-maal display-message -p -t alfa '#{window_index}'
done

echo "=== klikk på fane 1 viser vindu 1 ==="
tmux -L nerdr-vert send-keys -t vert -l "$(printf '\033[<0;46;1M\033[<0;46;1m')"
sleep 0.5
tmux -L nerdr-maal display-message -p -t alfa '#{window_index}'
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1p'

echo "=== C-b w og Enter går inn i Claude-en i vindu 0 ==="
tmux -L nerdr-vert send-keys -t vert C-b w
sleep 0.3
tmux -L nerdr-vert send-keys -t vert Enter
sleep 0.5
tmux -L nerdr-maal display-message -p -t alfa '#{window_index}'

echo "=== C-b 1 og C-b & spør, y lukker vinduet, og bare fane 0 står igjen (1 vindu) ==="
tmux -L nerdr-vert send-keys -t vert C-b 1 C-b '&'
sleep 0.3
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '16p'
tmux -L nerdr-vert send-keys -t vert y
sleep 1
tmux -L nerdr-maal list-windows -t alfa | wc -l
tmux -L nerdr-vert capture-pane -p -t vert | sed -n '1,2p'
tmux -L nerdr-vert send-keys -t vert C-b w

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
