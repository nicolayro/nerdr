//! `nerdr forklar`: hvordan nerdr brukes, skrevet for en agent som skal
//! forklare det for brukeren eller gjøre noe for dem. Brukerne har ingen skill
//! for nerdr, så det agenten vet, er det den leser her, via pekeren i
//! `nerdr --help`.

use nerdr_kjerne::{bindinger, tmux};

use crate::bindinger::{Bindinger, tast};
use crate::grupper::glyf;
use crate::hjelp::MERKER;
use crate::ny;
use crate::taster::{PREFIKS_VARIABEL, Prefiks};

enum Tast {
    /// Etter prefikset, som i `hjelp::PREFIKSTASTER`.
    Prefiks(&'static str),
    Egen(&'static str),
}

use Tast::{Egen, Prefiks as P};

/// Det brukeren vil, med musen og med tastaturet. Musen står først, for de
/// fleste kan ikke tmux.
const OFTEST: [(&str, &str, &[Tast]); 17] = [
    ("Starte en ny Claude", "«+ ny sesjon» øverst i agentlista, søk fram mappa og trykk ⏎", &[P("f")]),
    ("Fortsette en tidligere samtale", "", &[P("r")]),
    ("Se på en Claude", "klikk på kortet i agentlista", &[Egen("↑"), Egen("↓")]),
    ("Gå inn og skrive", "klikk på kortet, eller i Claude-en til høyre", &[Egen("⏎")]),
    ("Tilbake til agentlista", "klikk i agentlista", &[P("w")]),
    ("Neste eller forrige Claude", "klikk på kortet i agentlista", &[P(")"), P("(")]),
    ("Lukke en Claude", "hold musen over kortet og klikk ×", &[P("&")]),
    ("Ny terminal ved siden av Claude-en", "+ etter fanene øverst", &[P("c")]),
    ("Dele skjermen", "", &[P("%"), P("\"")]),
    ("Bytte mellom fanene øverst", "klikk på fanen", &[P("n"), P("p")]),
    ("Kopiere tekst", "dra over teksten, den kopieres når du slipper", &[P("[")]),
    ("Rulle tilbake", "musehjulet", &[P("PgUp")]),
    ("Bredere eller smalere agentliste", "dra i streken mellom agentlista og Claude-en", &[]),
    ("Endre utseendet", "", &[P("b")]),
    ("Finne alt nerdr kan gjøre", "", &[P(":")]),
    ("Hjelp", "«hjelp»", &[P("?")]),
    ("Avslutte nerdr", "«lukk nerdr». Claude-ene kjører videre", &[Egen("C-c i agentlista"), P("d")]),
];

/// Prefikset fra `NERDR_PREFIKS`, ellers fra tmux, ellers `C-b`, som i TUI-et.
fn prefiks(socket: Option<&str>) -> String {
    let fra_tmux = || tmux::kjør(socket, &["show-options", "-gv", "prefix"]).ok().and_then(|p| Prefiks::fra_tmux(&p));
    Prefiks::fra_miljø().or_else(fra_tmux).unwrap_or_default().navn()
}

/// Med tastene fra tmux-oppsettet, som i hjelpen i TUI-et.
fn bindinger(socket: Option<&str>) -> Bindinger {
    let linjer = tmux::kjør(socket, &["list-keys", "-T", "prefix"]).unwrap_or_default();
    Bindinger::ny(&bindinger::les(&linjer.lines().map(String::from).collect::<Vec<_>>()))
}

fn taster(taster: &[Tast], prefiks: &str, bindinger: &Bindinger) -> String {
    let tekst: Vec<String> = taster
        .iter()
        .map(|t| match t {
            P(navn) => format!("{prefiks} {}", tast(navn).map_or_else(|| navn.to_string(), |k| bindinger.vis(&k))),
            Egen(tast) => tast.to_string(),
        })
        .collect();
    tekst.join(", ")
}

pub fn tekst(socket: Option<&str>) -> String {
    let p = prefiks(socket);
    let b = bindinger(socket);
    let merker: Vec<String> = MERKER.iter().map(|(m, hva)| format!("- {} {hva}", glyf(*m))).collect();
    let mut ut = format!(
        "# nerdr, forklart for en agent

Brukeren spør hvordan nerdr fungerer, hva de kan gjøre i nerdr, eller hvordan
de gjør noe bestemt der. Svar med det de vil få til, og hvordan.

## Slik svarer du

- Forklar hva brukeren får til, ikke hvordan nerdr eller tmux virker.
- Nevn knappen eller musen først, og tasten som et alternativ, som «Klikk
  «+ ny sesjon» øverst i agentlista, eller trykk {p} f.» Mange bruker bare musen.
- Første gang du nevner en tast: «{p} f» betyr trykk {p}, slipp, og trykk f.
- Unngå tmux-ord som pane, socket, tmux-sesjon og prefiks, med mindre brukeren
  bruker dem selv. Si Claude, agentliste, fane og terminal.
- Svar på det brukeren spurte om. Tabellen under er for deg, ikke for å gjentas.

## Hva nerdr er

nerdr viser alle Claude-ene til brukeren i agentlista til venstre. Den som er
valgt, vises til høyre, og brukeren går inn i den og skriver som vanlig.
Agentlista viser hvem som spør om noe, hvem som er ferdige og venter, og hvem som jobber,
så brukeren ser hvem som trenger dem. Claude-ene kjører videre når nerdr lukkes,
og er der neste gang.

nerdr startes med `nerdr` i en vanlig terminalfane. Bare Claude-er som er
startet i nerdr, vises, ikke de i en vanlig terminalfane.

Merkene i agentlista:

{}

## Det brukeren gjør oftest

| Brukeren vil | Med musen | Med tastaturet |
| --- | --- | --- |
",
        merker.join("\n")
    );
    for (hva, mus, t) in OFTEST {
        ut += &format!("| {hva} | {mus} | {} |\n", taster(t, &p, &b).replace('|', "\\|"));
    }
    ut += &format!(
        "
## Vanlige spørsmål

- **En Claude vises ikke i agentlista:** den er startet i en vanlig terminalfane.
  Avslutt den der, og fortsett samtalen i nerdr med «hent samtale» ({p} r).
- **Mappene i velgeren:** velgeren søker i mappene brukeren har brukt Claude
  i, og i alle mappene under {mapper}, utenom skjulte mapper og det som er
  bygget. Mappene settes i skallets oppstartsfil, kolonseparert:
  `export {mapper_variabel}=~/git:~/kode`.
- **En annen tast enn {p}:** `export {PREFIKS_VARIABEL}=<tast>`, som C-s eller
  C-Space, i skallets oppstartsfil. Uten den gjelder tasten fra tmux-oppsettet,
  ellers C-b.
- **Stoppe alle Claude-ene:** `tmux kill-server`. Spør brukeren først.

## Det du kan gjøre selv

- `nerdr ny <mappe>` starter en Claude i mappa, som «+ ny sesjon», og brukeren
  ser den i agentlista med en gang.
- `nerdr samtaler` viser tidligere samtaler med id, og `nerdr ny --fortsett <id>`
  fortsetter én i mappa den hørte til.
- `nerdr liste --json` viser Claude-ene. `merke` er `spør` når en spør om noe,
  `ulest` når den er ferdig og venter, og `jobber` eller `sett` ellers.
- Endre ikke skallets oppstartsfil eller tmux-oppsettet uten å spørre brukeren.

## Feil og ønsker

Sitter brukeren fast, gjør nerdr noe rart, eller vil de ha noe annerledes, si
at de kan lage en issue på https://github.com/nicolayro/nerdr/issues. Lag den
ikke selv.
",
        mapper = ny::mapper(),
        mapper_variabel = ny::MAPPER_VARIABEL,
    );
    ut
}

#[cfg(test)]
mod tester {
    use super::*;
    use crate::hjelp::PREFIKSTASTER;

    #[test]
    fn hver_prefikstast_står_i_hjelpen() {
        let i_hjelpen: Vec<&str> = PREFIKSTASTER.iter().flat_map(|(t, _)| t.split_whitespace()).collect();
        for (hva, _, taster) in OFTEST {
            for tast in taster {
                if let P(tast) = tast {
                    assert!(i_hjelpen.contains(tast), "{hva}: {tast}");
                }
            }
        }
    }

    #[test]
    fn hver_rad_har_mus_eller_tast() {
        for (hva, mus, taster) in OFTEST {
            assert!(!mus.is_empty() || !taster.is_empty(), "{hva}");
        }
    }

    #[test]
    fn tastene_står_med_prefikset() {
        assert_eq!(taster(&[P(")"), P("(")], "C-a", &Bindinger::default()), "C-a ), C-a (");
        assert_eq!(taster(&[Egen("C-c i agentlista"), P("d")], "C-b", &Bindinger::default()), "C-c i agentlista, C-b d");
    }
}
