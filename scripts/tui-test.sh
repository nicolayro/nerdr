#!/bin/sh
# Kjører nerdr inne i én tmux-server og lar den vise en annen, med et falskt
# sesjonsregister. Ingenting av det er brukerens eget. Bygg med `cargo build`
# først. Stopper ved første sjekk som feiler, skriver den ut med skjermen til
# nerdr og går ut med 1. Sjekkene bygger på hverandre, så de etter en feil
# sier lite.
HER=$(cd "$(dirname "$0")" && pwd)
ROT=$(cd "$HER/.." && pwd -P)
NERDR=$ROT/target/debug/nerdr
FALSK=$HER/falsk-claude.sh
KONFIG=$(mktemp -d)
# Egne servere for hver kjøring, så to kjøringer samtidig, som fra to
# worktrees, ikke dreper hverandres.
MAAL=nerdr-maal-$$
VERT=nerdr-vert-$$
TOM=nerdr-tom-$$
antall=0

maal() { tmux -L "$MAAL" "$@"; }
vert() { tmux -L "$VERT" "$@"; }
tast() { tmux -L "$VERT" send-keys -t vert "$@"; }
klikk() { tast -l "$(printf '\033[<0;%s;%sM\033[<0;%s;%sm' "$1" "$2" "$1" "$2")"; }
hjul() { tast -l "$(printf '\033[<%s;%s;%sM' "$1" "$2" "$3")"; }
# dra <fra x> <fra y> <til x> <til y>, med venstre knapp.
dra() { tast -l "$(printf '\033[<0;%s;%sM\033[<32;%s;%sM\033[<0;%s;%sm' "$1" "$2" "$3" "$4" "$3" "$4")"; }
skjerm() { vert capture-pane -p -t vert; }

# Kjører resten av argumentene på nytt til de lykkes, i opptil fire sekunder.
# Lista fra registeret leses hvert annet sekund, så det må være lenger enn det.
sjekk() {
  hva=$1
  shift
  antall=$((antall + 1))
  i=0
  until "$@" > /dev/null 2>&1; do
    i=$((i + 1))
    if [ $i -ge 80 ]; then
      echo "FEIL i sjekk $antall: $hva"
      echo "  ($*)"
      skjerm | sed 's/^/  │/'
      exit 1
    fi
    sleep 0.05
  done
}

linje_har() { skjerm | sed -n "$1p" | grep -Eq -- "$2"; }
linje_mangler() { ! linje_har "$@"; }
har() { skjerm | grep -Eq -- "$1"; }
mangler() { ! har "$1"; }
i_pane() { maal capture-pane -p -t "$1" | grep -qF -- "$2"; }
er() {
  forventet=$1
  shift
  [ "$("$@")" = "$forventet" ]
}
inneholder() { case $1 in *"$2"*) ;; *) return 1 ;; esac; }
linjer() {
  n=$1
  shift
  [ "$("$@" | wc -l | tr -d ' ')" = "$n" ]
}
tell() { skjerm | grep -o -- "$1" | wc -l | tr -d ' '; }
dimensjon() { maal display-message -p -t "$1" '#{window_width}x#{window_height}'; }
pane_indeks() { maal display-message -p -t alfa:0 '#{pane_index}'; }
vindu_indeks() { maal display-message -p -t alfa '#{window_index}'; }
zoom() { maal display-message -p -t alfa:0 '#{window_zoomed_flag}'; }
mapper() { maal list-panes -t "$1" -F '#{pane_current_path}' | sort -u; }
sett_er_tall() { maal show-options -p -t "$1" -v @nerdr-sett | grep -Eq '^[0-9]+$'; }
uten_klienter() { [ -z "$(maal list-clients)" ]; }
i_lista() { linje_har 16 '^ \? hjelp'; }
i_panen() { linje_har 16 '^ C-b \? hjelp'; }
nerdr() { CLAUDE_CONFIG_DIR=$KONFIG "$NERDR" "$@" 2>&1; }
gruppert() { nerdr -L "$MAAL" liste | awk 'NF == 1 { printf "%s: ", $1 } NF > 1 { printf "%s %s, ", $1, $3 }'; }
fra_json() { nerdr -L "$MAAL" liste --json | grep -o "\"$1\":\"[a-zæøå]*\"" | cut -d'"' -f4 | tr '\n' ' '; }
sett_i_json() { nerdr -L "$MAAL" liste --json | grep -o '"sett":[0-9]' | wc -l | tr -d ' '; }

# registrer <fil> <sesjon> <status> [flere JSON-felt]
#
# Tiden er i millisekunder, som fra Claude Code. Med hele sekunder kan en
# Claude bli ferdig før du sist var inne i den, og havne i FERDIG.
registrer() {
  pane=$(maal display-message -p -t "$2" '#{pane_id}')
  printf '{"tmux":"%s:@0.%s","status":"%s","updatedAt":%s%s}' \
    "$2" "$pane" "$3" "$(perl -MTime::HiRes=time -e 'printf "%d", time * 1000')" "$4" > "$KONFIG/sessions/$1.json"
}
# En sesjon som kommer og går, får nerdr til å liste med en gang, så testen
# slipper å vente på intervallet.
list_straks() {
  maal new-session -d -s puff /bin/cat
  maal kill-session -t puff
}

mkdir "$KONFIG/sessions"
# Først i PATH for nerdr, så det testen kopierer, ikke havner på utklippstavla.
mkdir "$KONFIG/bin"
printf '#!/bin/sh\ncat > "%s/kopiert"\n' "$KONFIG" > "$KONFIG/bin/pbcopy"
chmod +x "$KONFIG/bin/pbcopy"
# HOME for nerdr, så mappevelgeren viser disse mappene og ikke brukerens egne.
mkdir -p "$KONFIG/git/prosjekt.ny" "$KONFIG/git/annet" "$KONFIG/kode/tredje"
# En tidligere samtale i annet, og en tom en som ikke skal vises.
mkdir -p "$KONFIG/projects/annet"
printf '{"type":"user","cwd":"%s"}\n{"type":"ai-title","aiTitle":"Fiks testene"}\n' "$KONFIG/git/annet" > "$KONFIG/projects/annet/abc-123.jsonl"
printf '{"type":"user","cwd":"%s"}\n{"type":"last-prompt"}\n' "$KONFIG/git/annet" > "$KONFIG/projects/annet/tom.jsonl"
# Mappa til alfa og gamma, med samme navn og branch uansett hvilken worktree
# og branch testen kjøres fra.
mkdir -p "$KONFIG/nerdr/.git"
echo 'ref: refs/heads/main' > "$KONFIG/nerdr/.git/HEAD"
MAPPE=$(cd "$KONFIG/nerdr" && pwd -P)

# tmux lar socket-fila ligge igjen, og med nye navn hver gang ville de hopet seg opp.
rydd() {
  for server in "$MAAL" "$VERT" "$TOM"; do
    tmux -L "$server" kill-server 2>/dev/null
    rm -f "${TMUX_TMPDIR:-/tmp}/tmux-$(id -u)/$server"
  done
  rm -rf "$KONFIG"
}
trap rydd EXIT
trap 'exit 1' INT TERM

maal -f /dev/null new-session -d -s alfa -x 80 -y 20 -c "$MAPPE" "sh $FALSK"
maal new-session -d -s beta -x 80 -y 20 /bin/sh
maal new-session -d -s gamma -x 80 -y 20 /bin/cat
# Nye sesjoner får cat i stedet for et skall, så «claude» som skrives inn,
# bare blir stående.
maal set -g default-command /bin/cat

# alfa venter og gamma jobber. beta er et skall uten Claude og skal ikke vises.
registrer alfa alfa waiting ",\"cwd\":\"$MAPPE\""
registrer gamma gamma busy ",\"cwd\":\"$MAPPE\""

vert -f /dev/null new-session -d -s vert -x 110 -y 16 \
  "HOME=$KONFIG PATH=$KONFIG/bin:$PATH NERDR_MAPPER=~/git:$KONFIG/kode NERDR_RAMMER=bokser CLAUDE_CONFIG_DIR=$KONFIG $NERDR -L $MAAL; sleep 30"

sjekk "alfa står under VENTER med ●, med oppgaven som tittel" linje_har 2 '^▎●  Fiks testene +│'
sjekk "under står mappa og branchen, og <1m til høyre" linje_har 3 '^▎   nerdr · main +<1m │'
sjekk "fanen står på en strek som fortsetter overskriften med ┬" linje_har 1 '^ VENTER .*┬─ [^ ]+ ─'
sjekk "JOBBER treffer skillestreken med ┤" linje_har 5 '^ JOBBER .*┤'
sjekk "gamma står under JOBBER med ◐, og uten oppgave er mappa tittelen" linje_har 6 '^ ◐  nerdr +│'
sjekk "under gamma står branchen og <1m" linje_har 7 '^    main +<1m │'
sjekk "beta har ingen Claude og vises ikke" mangler beta
sjekk "alfa får størrelsen til boksen" er 68x13 dimensjon alfa

tast Enter
tast -l 'hei æ'
tast Enter
sjekk "Enter går inn i alfa, og det du skriver, havner der" i_pane alfa 'hei æ'

vert set-buffer -b test "$(printf 'linje én\n$HOME "to"')"
vert paste-buffer -p -d -b test -t vert
sjekk "innliming starter med ^[[200~" i_pane alfa '^[[200~linje én'
sjekk "innliming slutter med ^[[201~, uten at tmux utvider \$HOME" i_pane alfa '$HOME "to"^[[201~'

hjul 64 41 3
sjekk "musehjulet kommer fram i alfa, regnet fra boksen" i_pane alfa '^[[<64;2;1M'

# Denne venter på intervallet, ikke på list_straks.
registrer alfa alfa idle ",\"cwd\":\"$MAPPE\""
sjekk "alfa blir ferdig mens du er inne, og står fortsatt under VENTER, med ○" linje_har 2 '^▎○  Fiks testene'
registrer alfa alfa waiting ",\"cwd\":\"$MAPPE\""
list_straks

tast C-b w j
sjekk "C-b w og j velger gamma" linje_har 6 '^▎◐  nerdr '
sjekk "alfa venter igjen" linje_har 2 '^ ●  Fiks testene'

klikk 5 2
sjekk "klikk på andre rad går rett inn i alfa" linje_har 2 '^▎●  Fiks testene +│'
sjekk "inne i panen viser hintet prefikset" i_panen
tast C-b w
sjekk "C-b w går tilbake til lista" i_lista

registrer gamma gamma idle
list_straks
sjekk "gamma blir ferdig og havner under VENTER med ◆, og uten cwd står gamma:0" linje_har 5 '^ ◆  gamma:0 +│'
sjekk "uten branch står bare tiden under" linje_har 6 '^ +<1m │'
sjekk "streken mellom kortene i VENTER" linje_har 4 '┈'

tast j Enter
sjekk "j og Enter går inn i gamma" i_panen
tast C-b w
sjekk "gamma står under FERDIG med ○, nederst" linje_har 14 '^▎○  gamma:0'
sjekk "FERDIG treffer skillestreken" linje_har 13 '^ FERDIG .*┤'
sjekk "nerdr har skrevet når du var inne i gamma, i @nerdr-sett" sett_er_tall gamma

tast k
sjekk "k opp til alfa" linje_har 2 '^▎●  Fiks testene +│'
klikk 5 14
sjekk "klikk på gamma nederst velger den" linje_har 14 '^▎○  gamma:0'
sjekk "klikk på gamma går inn i den" i_panen
tast C-b w
sjekk "tilbake i lista" i_lista

klikk 3 16
sjekk "klikk på «? hjelp» viser hjelpen" har 'trykk en tast for å lukke'
tast x
sjekk "en tast lukker hjelpen" mangler 'trykk en tast for å lukke'
tast '?'
sjekk "? viser hjelpen med prefikstastene" har 'C-b c +nytt vindu i mappa til panen'
tast x
sjekk "en tast lukker hjelpen igjen" mangler 'trykk en tast for å lukke'

tast n
sjekk "n åpner popupen med 3 mapper" linje_har 2 'mappe › .*3/3'
sjekk "~/git/annet står først og er valgt" linje_har 3 '▎~/git/annet'
sjekk "så ~/git/prosjekt.ny" linje_har 4 '~/git/prosjekt\.ny'
sjekk "så mappa i kode" linje_har 5 '~/kode/tredje'
tast -l pny
sjekk "«pny» treffer bare prosjekt.ny" linje_har 2 'mappe › pny .*1/3'
sjekk "prosjekt.ny er valgt" linje_har 3 '▎~/git/prosjekt\.ny'

tast Enter
sjekk "Enter lager sesjonen prosjekt-ny i mappa" er "$(cd "$KONFIG/git/prosjekt.ny" && pwd -P)" mapper prosjekt-ny
sjekk "claude skrives inn i prosjekt-ny" i_pane prosjekt-ny claude
sjekk "nerdr er inne i prosjekt.ny, med ◌" linje_har 6 '^▎◌  prosjekt\.ny'
sjekk "kortet sier «starter …»" linje_har 7 'starter …'
sjekk "hintet viser at du er inne" i_panen

registrer ny prosjekt-ny idle ",\"cwd\":\"$KONFIG/git/prosjekt.ny\""
list_straks
sjekk "i registeret får prosjekt.ny vanlig merke" linje_har 6 '^▎○  prosjekt\.ny'
sjekk "og mister «starter …»" linje_mangler 7 'starter'

tast C-b w n
sjekk "popupen åpner igjen" har 'mappe ›'
tast -l pny
sjekk "«pny» igjen" linje_har 2 '1/3'
tast Enter
sjekk "samme mappe igjen gir et nytt vindu i prosjekt-ny" linjer 2 maal list-windows -t prosjekt-ny
sjekk "nerdr er inne i det nye vinduet" i_panen

tast C-b w C-r
sjekk "C-r viser tidligere samtaler" linje_har 1 'tidligere samtaler'
sjekk "bare samtalen med meldinger, 1/1" linje_har 2 '1/1'
sjekk "annet  Fiks testene er valgt" linje_har 3 '▎annet  Fiks testene +<1m'
tast C-r
sjekk "C-r bytter til mappene" linje_har 1 'ny claude'
tast C-r
sjekk "C-r bytter tilbake" linje_har 1 'tidligere samtaler'
tast Enter
sjekk "Enter lager sesjonen annet og skriver inn claude --resume abc-123" i_pane annet 'claude --resume abc-123'
sjekk "nerdr er inne i annet" i_panen

tast C-b w n
sjekk "popupen åpner" har 'mappe ›'
tast Escape
sjekk "Esc lukker popupen" mangler 'mappe ›'
sjekk "Esc går tilbake til lista" i_lista
sjekk "Esc lager ingen ny sesjon" linjer 5 maal list-sessions

tast k k k k k k k k
sjekk "k opp til alfa" linje_har 2 '^▎●  Fiks testene +│'
tast C-b
sjekk "C-b viser prefikstastene i hintlinja" linje_har 16 'C-b … w lista · c vindu · \? mer'

tast %
sjekk "% deler alfa" linjer 2 maal list-panes -t alfa:0
sjekk "to bokser med luft mellom" linje_har 2 '╮ ╭'
sjekk "begge panene står i nerdr-mappa" er "$MAPPE" mapper alfa:0
sjekk "tmux får plass til begge boksene" er 64x13 dimensjon alfa:0

tast Enter
tast -l 'til venstre'
sjekk "Enter går inn i Claude-en og ikke i den nye panen" i_pane alfa:0.0 'til venstre'
klikk 90 5
sjekk "klikk i høyre pane gjør den aktiv" er 1 pane_indeks
tast -l 'til høyre'
sjekk "det du skriver, havner i høyre pane" i_pane alfa:0.1 'til høyre'

for i in $(seq 1 30); do maal send-keys -t alfa:0.1 "linje $i" Enter; done
# cat ekkoer hver linje, og telleren er bare stabil når alt er kommet fram.
sjekk "bunnen av høyre boks viser linje 30, to ganger" er 2 tell 'linje 30'
for i in 1 2 3 4; do hjul 64 90 6; done
sjekk "fire hakk opp gir 12/… på toppen av rammen" linje_har 2 ' 12/[0-9]+ '
for i in 1 2; do hjul 65 90 6; done
sjekk "to hakk ned gir 6/…" linje_har 2 ' 6/[0-9]+ '
tast Enter
sjekk "Enter går tilbake til bunnen uten teller" linje_mangler 2 '[0-9]+/[0-9]+'
dra 85 4 100 6
sjekk "å dra i en pane uten mus kopierer linjene" grep -q '^linje [0-9]*$' "$KONFIG/kopiert"
sjekk "kopiert står på rammen" linje_har 2 ' kopiert '

tast C-b C-b
sjekk "C-b C-b sender ^B til panen" i_pane alfa:0.1 '^B'
tast C-b o
sjekk "C-b o gjør Claude-en aktiv igjen" er 0 pane_indeks

tast C-b z
sjekk "C-b z zoomer inn" er 1 zoom
sjekk "med zoom vises bare én boks" er 1 tell '╭'
tast C-b z
sjekk "C-b z igjen zoomer ut" er 0 zoom
sjekk "begge boksene er tilbake" er 2 tell '╭'

maal set-hook -g window-resized "run-shell 'echo #{session_name} #{window_width}x#{window_height} >> $KONFIG/storrelser'"
tast C-b ')'
sjekk "C-b ) går til neste Claude" linje_mangler 2 '^▎'
tast C-b '('
sjekk "C-b ( går tilbake til alfa" linje_har 2 '^▎●  Fiks testene +│'
# Et vindu som endrer størrelse, gjør det like etter byttet.
sleep 0.5
sjekk "ingen vindu endrer størrelse når du bytter Claude" [ ! -s "$KONFIG/storrelser" ]
maal set-hook -gu window-resized

tast C-b o C-b x
sjekk "C-b x spør" linje_har 16 'lukke panen\? y/n'
tast n
sjekk "n lar panen stå" linje_mangler 16 'lukke'
sjekk "begge panene er der" linjer 2 maal list-panes -t alfa:0
tast C-b x y
sjekk "C-b x y lukker panen" linjer 1 maal list-panes -t alfa:0

tast C-b '"'
sjekk "C-b \" deler over og under" er 2 tell '╭'
tast C-b x y
sjekk "C-b x y lukker den nye" linjer 1 maal list-panes -t alfa:0
sjekk "uten split fyller Claude-en hele visningen igjen" er 68x13 dimensjon alfa:0
tast C-b w

tast C-b B
sjekk "C-b B gir linjer, og alfa blir større" er 74x15 dimensjon alfa
sjekk "med linjer er det ingen bokser" mangler '╭'
tast C-b B
sjekk "C-b B igjen gir bokser" er 68x13 dimensjon alfa
sjekk "boksen er tilbake" har '╭'

tast C-b c
sjekk "C-b c fra lista lager alfa:1" linjer 2 maal list-windows -t alfa
sjekk "to faner øverst" linje_har 1 '┬─ [^ ]+ ─ [^ ]+ ─'
sjekk "nerdr er inne i det nye vinduet" i_panen
tast -l 'i vindu 1'
sjekk "det du skriver, havner i alfa:1" i_pane alfa:1 'i vindu 1'

tast C-b n
sjekk "C-b n går rundt til 0" er 0 vindu_indeks
tast C-b l
sjekk "C-b l går tilbake til 1" er 1 vindu_indeks
tast C-b 0
sjekk "C-b 0 går til 0" er 0 vindu_indeks
klikk 46 1
sjekk "klikk på fane 1 viser vindu 1" er 1 vindu_indeks
tast C-b w Enter
sjekk "C-b w og Enter går inn i Claude-en i vindu 0" er 0 vindu_indeks

tast C-b 1 C-b '&'
sjekk "C-b & spør" linje_har 16 'lukke vinduet\? y/n'
tast y
sjekk "y lukker vinduet" linjer 1 maal list-windows -t alfa
sjekk "bare én fane står igjen" linje_mangler 1 '┬─ [^ ]+ ─ [^ ]+ ─'
tast C-b w

tast q
sjekk "q avslutter" uten_klienter

sjekk "nerdr liste grupperer som TUI-et: alfa venter, og prosjekt.ny og gamma er sett, nyest først" \
  er 'VENTER: ● nerdr, FERDIG: ○ prosjekt.ny, ○ gamma:0, ' gruppert
sjekk "nerdr liste --json har tilstanden fra Claude" er 'venter klar klar ' fra_json tilstand
sjekk "nerdr liste --json har merket" er 'spør sett sett ' fra_json merke
sjekk "nerdr liste --json har gruppen" er 'venter ferdig ferdig ' fra_json gruppe
sjekk "nerdr liste --json har versjon" inneholder "$(nerdr -L "$MAAL" liste --json)" '"versjon":1'
sjekk "alle tre har sett med et tall" er 3 sett_i_json
sjekk "nerdr samtaler viser Fiks testene i annet, med id" inneholder "$(nerdr samtaler)" 'abc-123  annet  Fiks testene'

ut=$(nerdr -L "$MAAL" ny "$KONFIG/kode/tredje")
sjekk "nerdr ny lager sesjonen tredje" inneholder "$ut" 'tmux-sesjonen tredje'
sjekk "og skriver inn claude" i_pane tredje claude
ut=$(nerdr -L "$MAAL" ny --fortsett abc-123 --json)
sjekk "nerdr ny --fortsett finner mappa annet" inneholder "$ut" '"sesjon":"annet"'
sjekk "og gir et nytt vindu der" linjer 2 maal list-windows -t annet
sjekk "med claude --resume" i_pane annet:1 'claude --resume abc-123'

registrer tredje tredje busy ',"sessionId":"abc-123"'
sjekk "en samtale som kjører, startes ikke en gang til" inneholder "$(nerdr -L "$MAAL" ny --fortsett abc-123)" 'samtalen abc-123 kjører alt'
sjekk "ukjent id gir feil" inneholder "$(nerdr -L "$MAAL" ny --fortsett finnes-ikke)" 'fant ingen samtale med id finnes-ikke'
sjekk "en id som ikke er bokstaver, tall og -, skrives ikke inn i skallet" \
  inneholder "$(nerdr -L "$MAAL" ny "$KONFIG/kode/tredje" --fortsett 'abc; ls')" 'er ikke en id for en samtale'
maal kill-server

vert new-session -d -s tom "CLAUDE_CONFIG_DIR=$KONFIG $NERDR -L $TOM; sleep 30"
sjekk "uten tmux-server lager nerdr en sesjon" linjer 1 tmux -L "$TOM" list-sessions
sjekk "uten tmux sier nerdr fra" inneholder "$(PATH=/finnes-ikke "$NERDR" 2>&1)" 'fant ikke tmux'

echo "$antall sjekker, alle grønne"
