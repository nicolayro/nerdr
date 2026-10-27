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
beveg() { tast -l "$(printf '\033[<35;%s;%sM' "$1" "$2")"; }
# dra <fra x> <fra y> <til x> <til y>, med venstre knapp.
dra() { tast -l "$(printf '\033[<0;%s;%sM\033[<32;%s;%sM\033[<0;%s;%sm' "$1" "$2" "$3" "$4" "$3" "$4")"; }
skjerm() { vert capture-pane -p -t vert; }
vert_har() { vert capture-pane -p -t "$1" | grep -Eq -- "$2"; }

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
linje_med_farger_har() { vert capture-pane -p -e -t vert | sed -n "$1p" | grep -Fq -- "$2"; }
# Panen ved siden av har sitt eget oransje ●, men aldri et grønt ◐.
gronn_glyf() { linje_med_farger_har "$1" '38;5;108m◐'; }
uten_gronn_glyf() { ! gronn_glyf "$1"; }
har() { skjerm | grep -Eq -- "$1"; }
mangler() { ! har "$1"; }
i_pane() { maal capture-pane -p -t "$1" | grep -qF -- "$2"; }
ikke_i_pane() { ! i_pane "$@"; }
# Kolonnen, fra 1, der teksten starter på linja, regnet i tegn.
kolonne() { skjerm | sed -n "$1p" | perl -CSA -e '$s = shift; while (<STDIN>) { print index($_, $s) + 1 }' "$2"; }
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
uten_klienter() { [ -z "$(tmux -L "${1:-$MAAL}" list-clients)" ]; }
klienter() { maal list-clients | wc -l | tr -d ' '; }
uten_sesjon() { ! maal has-session -t "=$1" 2>/dev/null; }
# Linja, fra 1, der teksten står først på skjermen.
linja_med() { skjerm | grep -n -- "$1" | head -1 | cut -d: -f1; }
# Tastene går til lista når nerdr ikke viser markøren til panen.
i_lista() { er 0 vert display-message -p -t vert '#{cursor_flag}'; }
i_panen() { er 1 vert display-message -p -t vert '#{cursor_flag}'; }
nerdr() { CLAUDE_CONFIG_DIR=$KONFIG "$NERDR" "$@" 2>&1; }
gruppert() { nerdr -L "$MAAL" liste | awk 'NF == 1 { printf "%s: ", $1 } NF > 1 { printf "%s %s, ", $1, $3 }'; }
fra_json() { nerdr -L "$MAAL" liste --json | grep -o "\"$1\":\"[a-zæøå]*\"" | cut -d'"' -f4 | tr '\n' ' '; }
sett_i_json() { nerdr -L "$MAAL" liste --json | grep -o '"sett":[0-9]' | wc -l | tr -d ' '; }

# registrer <fil> <sesjon> <status> [flere JSON-felt]
#
# Tiden er i millisekunder, som fra Claude Code. Med hele sekunder kan en
# Claude bli ferdig før du sist var inne i den, og bli ledig.
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
# Et prosjekt dypere nede, som en app i et monorepo, og et som er lastet ned.
mkdir -p "$KONFIG/git/annet/apps/web/node_modules/pakke"
touch "$KONFIG/git/annet/apps/web/package.json" "$KONFIG/git/annet/apps/web/node_modules/pakke/package.json"
# En samtale uten meldinger i en mappe utenfor NERDR_MAPPER, som mappevelgeren
# viser. Den skrives først, så annet er brukt sist.
mkdir -p "$KONFIG/eget" "$KONFIG/projects/eget"
printf '{"type":"user","cwd":"%s"}\n' "$KONFIG/eget" > "$KONFIG/projects/eget/tom.jsonl"
# En tidligere samtale i annet, og en tom en som ikke skal vises.
mkdir -p "$KONFIG/projects/annet"
printf '{"type":"user","cwd":"%s"}\n{"type":"ai-title","aiTitle":"Fiks testene"}\n' "$KONFIG/git/annet" > "$KONFIG/projects/annet/abc-123.jsonl"
printf '{"type":"user","cwd":"%s"}\n{"type":"last-prompt"}\n' "$KONFIG/git/annet" > "$KONFIG/projects/annet/tom.jsonl"
# Mappa til alfa og gamma, med samme navn og branch uansett hvilket worktree
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
# alfa registreres sist, så den står øverst.
registrer gamma gamma busy ",\"cwd\":\"$MAPPE\""
registrer alfa alfa waiting ",\"cwd\":\"$MAPPE\""
maal set -g @nerdr-utseende paner=bokser

vert -f /dev/null new-session -d -s vert -x 110 -y 20 \
  "HOME=$KONFIG PATH=$KONFIG/bin:$PATH NERDR_MAPPER=~/git:$KONFIG/kode CLAUDE_CONFIG_DIR=$KONFIG $NERDR -L $MAAL; sleep 30"

sjekk "paner=bokser fra tmux gir bokser, under streken øverst" linje_har 1 '^─+┬─ [^ ]+ ─'
sjekk "overskriften har strek mot skillelinja, uten antallet" linje_har 2 '^ AKTIVE ─+┤'
sjekk "alfa har ●, repoet og <1m i toppkanten" linje_har 3 '^╭─ ●  nerdr ─+ <1m ─╮'
beveg 10 4
sjekk "med musen over kortet står × i toppkanten i stedet for tiden" linje_har 3 '^╭─ ●  nerdr ─+  ×  ─╮'
beveg 60 8
sjekk "og tiden er tilbake når musen går" linje_har 3 '<1m ─╮'
sjekk "så oppgaven" linje_har 4 '^│ Fiks testene +│'
sjekk "og branchen" linje_har 5 '^│ main +│'
sjekk "gamma står under alfa, og uten oppgave er repoet tittelen" linje_har 8 '^│ nerdr +│'
sjekk "◐ til gamma er grønn" gronn_glyf 7
tast C-b b
sjekk "C-b b åpner utseende-menyen" har 'farge +av +på'
tast Left
sjekk "farge av tar fargen bort" uten_gronn_glyf 7
tast Right Down Down Right
sjekk "orden mappe gir repoet som overskrift" linje_har 2 '^ nerdr {3,}'
sjekk "og bare branchen under oppgaven" linje_har 5 '^│ main +│'
tast Right
sjekk "orden aktivitet har ingen overskrifter" mangler 'AKTIVE|LEDIGE'
tast Right Up Right Escape
sjekk "Esc lukker menyen" mangler 'farge +av'
sjekk "resten av testen bruker minimal" er 'paner=bokser farge=på plass=minimal orden=nerdr ledig=under tittel=1' maal show-options -gqv @nerdr-utseende

sjekk "minimal har én linje: alfa under AKTIVE med ●, oppgaven og <1m til høyre" linje_har 3 '^▎ ●  Fiks testene +<1m │'
sjekk "overskriftene treffer skillestreken med ┤" linje_har 2 '^ AKTIVE ─+┤'
sjekk "gamma står under alfa med ◐, og uten oppgave er repoet tittelen" linje_har 4 '^  ◐  nerdr +<1m │'
sjekk "beta har ingen Claude og vises ikke" mangler beta
sjekk "alfa får størrelsen til boksen" er 68x17 dimensjon alfa

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
sjekk "alfa blir ferdig mens du er inne, og er fortsatt aktiv, med ◆" linje_har 3 '^▎ ◆  Fiks testene'
registrer alfa alfa waiting ",\"cwd\":\"$MAPPE\""
list_straks

tast C-b w j
sjekk "C-b w og j velger gamma" linje_har 4 '^▎ ◐  nerdr '
sjekk "alfa venter igjen" linje_har 3 '^  ●  Fiks testene'

klikk 5 3
sjekk "klikk på alfa velger den, og musen over den viser ×" linje_har 3 '^▎ ●  Fiks testene +×  │'
sjekk "og du blir i lista" i_lista
klikk 5 3
sjekk "et klikk til på samme kort går inn i alfa" i_panen
klikk 5 14
sjekk "klikk i lista utenfor kortene går ut" i_lista
klikk 60 8
sjekk "klikk i panen går inn" i_panen
sjekk "uten at klikket kommer fram i alfa" ikke_i_pane alfa '^[[<0;'
tast C-b w
sjekk "C-b w går tilbake til lista" i_lista

registrer gamma gamma idle
list_straks
sjekk "gamma blir ferdig og havner øverst under AKTIVE med ◆, og uten cwd står gamma:0" linje_har 3 '^  ◆  gamma:0 +<1m │'
sjekk "alfa står rett under" linje_har 4 '^▎ ●  Fiks testene'

tast k Enter
sjekk "k og Enter går inn i gamma" i_panen
tast C-b w
sjekk "når du går ut, står gamma under LEDIGE med ○" linje_har 6 '^▎ ○  gamma:0'
sjekk "LEDIGE treffer skillestreken" linje_har 5 '^ LEDIGE .*┤'
sjekk "nerdr har skrevet når du var inne i gamma, i @nerdr-sett" sett_er_tall gamma

tast Up
sjekk "↑ opp til alfa" linje_har 3 '^▎ ●  Fiks testene +<1m │'
tast Down q n
sjekk "q og n gjør ingenting i lista" linje_har 6 '^▎ ○  gamma:0'
klikk 5 6
sleep 0.6
klikk 5 6
sjekk "to klikk med pause mellom går ikke inn" i_lista
klikk 5 6
sjekk "dobbeltklikk på gamma går inn i den" i_panen
klikk 5 3
sjekk "klikk på alfa fra panen velger alfa og går ut i lista" linje_har 3 '^▎ ●  Fiks testene'
sjekk "tilbake i lista" i_lista

sjekk "nederst i lista er det en strek" linje_har 16 '^─+┤'
sjekk "«+ ny sesjon» først" linje_har 17 '^ \+ ny sesjon +C-b f │'
sjekk "så «handlinger»" linje_har 18 '^   handlinger +C-b : │'
sjekk "så «hjelp»" linje_har 19 '^   hjelp +C-b \? │'
sjekk "og «avslutt» nederst" linje_har 20 '^   avslutt +C-b d │'
klikk 3 19
sjekk "klikk på «hjelp» viser hjelpen" har 'trykk en tast for å lukke'
tast x
sjekk "en tast lukker hjelpen" mangler 'trykk en tast for å lukke'
tast C-b '?'
sjekk "C-b ? viser hjelpen med C-b f" har 'C-b f +ny sesjon'
tast x
sjekk "en tast lukker hjelpen igjen" mangler 'trykk en tast for å lukke'
tast C-b :
sjekk "C-b : fra lista åpner handlingene" har '⏎ kjør · esc lukk'
tast Escape
sjekk "Esc lukker dem" mangler '⏎ kjør'

klikk 3 17
sjekk "klikk på «+ ny sesjon» åpner popupen med 4 mapper og 1 prosjekt" linje_har 3 'mappe › .*5/5'
sjekk "~/git/annet er brukt sist, står først og er valgt" linje_har 4 '▎~/git/annet'
sjekk "så ~/eget, som er brukt før" linje_har 5 '~/eget'
sjekk "så ~/git/prosjekt.ny" linje_har 6 '~/git/prosjekt\.ny'
sjekk "så mappa i kode" linje_har 7 '~/kode/tredje'
sjekk "og prosjektet i annet, men ikke det i node_modules" linje_har 8 '~/git/annet/apps/web +│'
tast -l pny
sjekk "«pny» treffer bare prosjekt.ny" linje_har 3 'mappe › pny .*1/5'
sjekk "prosjekt.ny er valgt" linje_har 4 '▎~/git/prosjekt\.ny'

tast Enter
sjekk "Enter lager sesjonen prosjekt-ny i mappa" er "$(cd "$KONFIG/git/prosjekt.ny" && pwd -P)" mapper prosjekt-ny
sjekk "claude skrives inn i prosjekt-ny" i_pane prosjekt-ny claude
sjekk "nerdr er inne i prosjekt.ny, øverst med ◌" linje_har 3 '^▎ ◌  prosjekt\.ny'
sjekk "og tastene går dit" i_panen

registrer ny prosjekt-ny idle ",\"cwd\":\"$KONFIG/git/prosjekt.ny\""
list_straks
sjekk "i registeret får prosjekt.ny vanlig merke, og er fortsatt aktiv" linje_har 3 '^▎ ○  prosjekt\.ny'

tast C-b f
sjekk "C-b f åpner popupen fra panen" har 'mappe ›'
tast -l pny
sjekk "«pny» igjen" linje_har 3 '1/5'
tast Enter
sjekk "samme mappe igjen gir et nytt vindu i prosjekt-ny" linjer 2 maal list-windows -t prosjekt-ny
sjekk "nerdr er inne i det nye vinduet" i_panen

tast C-b r
sjekk "C-b r viser samtalene" linje_har 2 'hent samtale'
sjekk "bare samtalen med meldinger, 1/1" linje_har 3 '1/1'
sjekk "annet  Fiks testene er valgt" linje_har 4 '▎annet  Fiks testene +<1m'
tast C-r
sjekk "C-r bytter til mappene" linje_har 2 'ny sesjon'
tast C-r
sjekk "C-r bytter tilbake" linje_har 2 'hent samtale'
tast Enter
sjekk "Enter lager sesjonen annet og skriver inn claude --resume abc-123" i_pane annet 'claude --resume abc-123'
sjekk "nerdr er inne i annet" i_panen

tast C-b f
sjekk "popupen åpner" har 'mappe ›'
tast Escape
sjekk "Esc lukker popupen" mangler 'mappe ›'
sjekk "Esc går tilbake til lista" i_lista
sjekk "Esc lager ingen ny sesjon" linjer 5 maal list-sessions

# De to som starter, står øverst blant de aktive, over alfa.
tast Up Up Up Up Up Up Up Up Down Down
sjekk "↑ til toppen og ↓ ned til alfa" linje_har 5 '^▎ ●  Fiks testene +<1m │'

klikk 3 18
sjekk "klikk på «handlinger» åpner dem" har '⏎ kjør · esc lukk'
tast -l terminal
tast Enter
sjekk "«ny terminal» der gir en terminal i et nytt vindu i alfa" linjer 2 maal list-windows -t alfa
sjekk "i mappa til alfa" er "$MAPPE" mapper alfa:1
sjekk "og du er inne i den" i_panen
tast C-b '&' y
sjekk "lukket igjen" linjer 1 maal list-windows -t alfa

tast C-b w
klikk "$(kolonne 1 ' + ')" 1
sjekk "klikk på + etter fanene gir en terminal i et nytt vindu" linjer 2 maal list-windows -t alfa
sjekk "i mappa til alfa" er "$MAPPE" mapper alfa:1
sjekk "slippet etter klikket kommer ikke fram i alfa" ikke_i_pane alfa '^[[<0;'
tast C-b '&' y
sjekk "lukket igjen" linjer 1 maal list-windows -t alfa
tast C-b w
tast C-b
sjekk "C-b viser prefikstastene i hintlinja under streken" linje_har 17 'C-b … f ny · w lista · \? mer'
sjekk "og knappene er borte så lenge" mangler 'avslutt +C-b d'

tast %
sjekk "% deler alfa" linjer 2 maal list-panes -t alfa:0
sjekk "to bokser med luft mellom" linje_har 2 '╮ ╭'
sjekk "begge panene står i nerdr-mappa" er "$MAPPE" mapper alfa:0
sjekk "tmux får plass til begge boksene" er 64x17 dimensjon alfa:0

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

tast C-b :
sjekk "C-b : fra panen åpner handlingene" har '⏎ kjør · esc lukk'
tast -l zoom
sjekk "«zoom» viser kommandoen med tasten" har 'zoom pane +C-b z'
tast Enter
sjekk "Enter kjører den" er 1 zoom
klikk 3 18
sjekk "klikk på «handlinger» fra panen åpner dem" har '⏎ kjør · esc lukk'
tast -l zoom
tast Enter
sjekk "og zoom igjen zoomer ut" er 0 zoom

maal set-hook -g window-resized "run-shell 'echo #{session_name} #{window_width}x#{window_height} >> $KONFIG/storrelser'"
tast C-b ')'
sjekk "C-b ) går til neste Claude" linje_mangler 2 '^▎'
tast C-b '('
sjekk "C-b ( går tilbake til alfa" linje_har 5 '^▎ ●  Fiks testene +<1m │'
# Et vindu som endrer størrelse, gjør det like etter byttet.
sleep 0.5
sjekk "ingen vindu endrer størrelse når du bytter Claude" [ ! -s "$KONFIG/storrelser" ]
maal set-hook -gu window-resized

tast C-b o C-b x
sjekk "C-b x spør i en popup" har 'lukke panen\? '
sjekk "med «lukk» og «avbryt»" har ' lukk  y +avbryt  n '
tast n
sjekk "n lar panen stå" mangler 'lukke panen'
sjekk "begge panene er der" linjer 2 maal list-panes -t alfa:0
tast C-b x y
sjekk "C-b x y lukker panen" linjer 1 maal list-panes -t alfa:0

tast C-b '"'
sjekk "C-b \" deler over og under" er 2 tell '╭'
tast C-b x y
sjekk "C-b x y lukker den nye" linjer 1 maal list-panes -t alfa:0
sjekk "uten split fyller Claude-en hele visningen igjen" er 68x17 dimensjon alfa:0
tast C-b w

tast C-b b Up Right Escape
sjekk "linjer rundt panene i menyen gjør alfa større" er 74x19 dimensjon alfa
sjekk "med linjer er det ingen bokser" mangler '╭'
tast C-b b Up Left Escape
sjekk "og bokser igjen" er 68x17 dimensjon alfa
sjekk "boksen er tilbake" har '╭'

tast C-b c
sjekk "C-b c fra lista lager alfa:1" linjer 2 maal list-windows -t alfa
sjekk "to faner øverst" linje_har 1 '┬─ [^ ]+ ─ [^ +][^ ]* ─'
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
sjekk "C-b & spør" har 'lukke vinduet\? '
tast y
sjekk "y lukker vinduet" linjer 1 maal list-windows -t alfa
sjekk "bare én fane står igjen" linje_mangler 1 '┬─ [^ ]+ ─ [^ +][^ ]* ─'
tast C-b w

tast C-b w Up Up Up Up Up Up Up Up
for i in 1 2 3 4 5 6 7 8 9; do
  maal new-session -d -s "ekstra$i" -x 80 -y 20 -c "$MAPPE"
  registrer "ekstra$i" "ekstra$i" idle ",\"cwd\":\"$MAPPE\""
done
list_straks
sjekk "får ikke lista plass, har nederste rad en pil" linje_har 15 '^  ↓ +│'
sjekk "og lista starter med luft under streken" linje_har 3 '^ AKTIVE'
sjekk "øverst er det ingen pil" linje_mangler 1 '↑'
for i in 1 2 3 4; do hjul 65 5 5; done
sjekk "hjulet ruller lista, og øverst er det en pil" linje_har 1 '^─ ↑ ─'
sjekk "som er oransje fordi alfa over spør" linje_med_farger_har 1 '38;2;217;119;87m ↑'
tast Down Down Down Down Down Down Down Down Down Down Down Down Down Down
sjekk "den valgte nederst i lista vises" har '^▎ ○'
sjekk "nederst er det ingen pil" linje_mangler 15 '↓'
klikk 3 1
sjekk "klikk på pilen ruller en side opp" linje_har 15 '↓'
for i in 1 2 3 4 5 6 7 8 9; do
  maal kill-session -t "ekstra$i"
  rm "$KONFIG/sessions/ekstra$i.json"
done
list_straks
sjekk "nerdr kobler til igjen når sesjonen den viste, forsvinner" er 1 klienter
sjekk "når alt får plass igjen, er pilene borte" mangler ' [↑↓] '
tast Up Up Up Up Up Up Up Up Down Down

maal new-session -d -s lukk -x 80 -y 20 -c "$MAPPE"
maal select-pane -t lukk -T '✳ Lukk meg'
registrer lukk lukk idle ",\"cwd\":\"$MAPPE\""
maal split-window -d -t lukk:0 -c "$MAPPE"
maal select-pane -t lukk:0.1 -T '✳ Lukk nummer to'
registrer lukk2 lukk:0.1 idle ",\"cwd\":\"$MAPPE\""
list_straks
sjekk "to Claude-er i samme vindu" har 'Lukk nummer to'
rad=$(linja_med 'Lukk meg')
sjekk "uten musen over kortet er det ingen ×" linje_mangler "$rad" '×'
beveg 10 "$rad"
sjekk "med musen over kortet står × der tiden var" linje_har "$rad" 'Lukk meg +×  │'
klikk 34 "$rad"
sjekk "klikk ved siden av × spør" har 'lukke sesjonen\? '
sjekk "om bare Claude-en, når en til står i vinduet" har '«Lukk meg»\. De andre Claude-ene i vinduet'
tast n
sjekk "n lukker popupen" mangler 'lukke sesjonen'
sjekk "og lar Claude-en stå" har 'Lukk meg'
klikk 33 "$rad"
svar=$(linja_med ' avbryt ')
klikk "$(kolonne "$svar" ' avbryt ')" "$svar"
sjekk "klikk på «avbryt» lukker popupen" mangler 'lukke sesjonen'
sjekk "og lar panen stå" linjer 2 maal list-panes -t =lukk:0
klikk 33 "$rad"
klikk "$(kolonne "$svar" ' lukk ')" "$svar"
sjekk "klikk på «lukk» lukker bare panen" linjer 1 maal list-panes -t =lukk:0
sjekk "og kortet er borte" mangler 'Lukk meg'
rm "$KONFIG/sessions/lukk.json"
rad=$(linja_med 'Lukk nummer to')
beveg 10 "$rad"
klikk 33 "$rad"
sjekk "alene i vinduet spør den om vinduet" har '«Lukk nummer to» og vinduet den står i\.'
tast y
sjekk "y lukker vinduet, og sesjonen med det" uten_sesjon lukk
rm "$KONFIG/sessions/lukk2.json"

dra 36 8 46 8
sjekk "å dra i streken gjør lista bredere" linje_har 5 '^▎ ●  Fiks testene +<1m +│ '
sjekk "og alfa smalere" er 58x17 dimensjon alfa
sjekk "bredden står i tmux" er 46 maal show-options -gqv @nerdr-liste
klikk 46 8
klikk 46 8
sjekk "dobbeltklikk på streken gjør lista til en stripe" linje_har 4 '^▎● │ '
sjekk "med bare tegnene" mangler 'Fiks testene'
sjekk "også i tmux" er '46 stripe' maal show-options -gqv @nerdr-liste
klikk 4 8
klikk 4 8
sjekk "dobbeltklikk igjen gir bredden tilbake" er 58x17 dimensjon alfa
dra 46 8 2 8
sjekk "å dra streken helt inn gir stripa" linje_har 4 '^▎● │ '
tast C-b
sjekk "fra stripa står hintet over panen" linje_har 20 '^ C-b … f ny · w lista · \? mer'
tast w
dra 4 8 36 8
sjekk "og å dra den ut gir lista tilbake" er 68x17 dimensjon alfa

klikk 5 20
sjekk "klikk på «avslutt» avslutter" uten_klienter

sjekk "nerdr liste grupperer som TUI-et: alfa er aktiv, og gamma og prosjekt.ny er ledige, nyest først" \
  er 'AKTIVE: ● nerdr, LEDIGE: ○ prosjekt.ny, ○ gamma:0, ' gruppert
sjekk "nerdr liste --json har tilstanden fra Claude" er 'venter klar klar ' fra_json tilstand
sjekk "nerdr liste --json har merket" er 'spør sett sett ' fra_json merke
sjekk "nerdr liste --json har gruppen" er 'venter ferdig ferdig ' fra_json gruppe
sjekk "nerdr liste --json har repoet der det er et" er 'nerdr ' fra_json repo
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

# Den nye serveren gir panene de samme id-ene, så Claude-ene fra før ville
# stått i lista. Igjen er bare en Claude i en vanlig terminal, med testen som
# prosessen.
rm "$KONFIG"/sessions/*.json
printf '{"kind":"interactive","status":"idle"}' > "$KONFIG/sessions/$$.json"
vert new-session -d -s tom "NERDR_PREFIKS=C-a CLAUDE_CONFIG_DIR=$KONFIG $NERDR -L $TOM; sleep 30"
sjekk "uten tmux-server lager nerdr en sesjon" linjer 1 tmux -L "$TOM" list-sessions
sjekk "uten Claude forklarer nerdr hva lista er" vert_har tom 'Her er det tomt foreløpig'
sjekk "og at én Claude kjører utenfor tmux" vert_har tom 'Du har én Claude åpen i en vanlig'
vert send-keys -t tom C-a
sjekk "NERDR_PREFIKS gir et annet prefiks, og hintet viser det" vert_har tom 'C-a … f ny · w lista'
vert send-keys -t tom w q C-a d
sjekk "q gjør ingenting, og C-a d avslutter" uten_klienter "$TOM"
sjekk "uten tmux sier nerdr fra" inneholder "$(PATH=/finnes-ikke "$NERDR" 2>&1)" 'fant ikke tmux'

echo "$antall sjekker, alle grønne"
