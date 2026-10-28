//! Kortene i lista og overskriftene over gruppene, etter utseendet. Et kort er
//! linjer med tekstbiter, så høyden er kjent før det tegnes, og et klikk kan
//! treffe riktig kort.

use nerdr_kjerne::merke::{Gruppe, Merke};
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Style};

use super::{GRØNN, GRØNN_RGB, ORANSJE, ORANSJE_RGB, STERK, SVAK, VANLIG, bryt, kutt, tonet};
use crate::grupper::{Overskrift, Rad, glyf};
use crate::utseende::{Farge, Plass, Utseende};

/// Uten glyf foran starter teksten etter pekeren og et mellomrom.
const INNRYKK: u16 = 2;
/// Teksten etter glyfen i minimal. Glyfene har tvetydig bredde i Unicode, og
/// mange fonter tegner dem over to kolonner.
const ETTER_GLYF: u16 = INNRYKK + 3;
/// Tonen av køens farge i rammer og streker.
const TONE: f32 = 0.5;

pub struct Info {
    pub merke: Merke,
    pub oppgave: Option<String>,
    pub sted: String,
    pub branch: Option<String>,
    /// Hvor lenge den har ventet eller jobbet.
    pub tid: Option<String>,
    /// Startet av nerdr og ikke i registeret ennå.
    pub starter: bool,
    /// Lista er gruppert etter mappe, så repoet står i overskriften.
    pub sted_over: bool,
}

pub struct Bit {
    pub x: u16,
    pub tekst: String,
    pub stil: Style,
}

pub type Linje = Vec<Bit>;

/// Tekstbitene på en side av en kant.
type Kanttekst = Vec<(String, Style)>;

fn bit(x: u16, tekst: impl Into<String>, stil: Style) -> Bit {
    Bit { x, tekst: tekst.into(), stil }
}

fn bredde(tekst: &str) -> u16 {
    tekst.chars().count() as u16
}

fn peker(valgt: bool) -> Bit {
    bit(0, if valgt { "▎" } else { " " }, VANLIG)
}

/// Lista starter under den øverste raden, der fanene står.
pub fn ordne(rader: Vec<Rad<'_>>) -> Vec<Rad<'_>> {
    let mut ut = vec![Rad::Tom];
    ut.extend(rader);
    ut
}

/// Overskriften over en gruppe, der `bredde` er lista uten skillelinja. Gir
/// om streken går til skillelinja.
pub fn overskrift(u: Utseende, buf: &mut Buffer, y: u16, bredde: u16, o: &Overskrift, stripe: bool) -> bool {
    let b = bredde as usize;
    let strek = |buf: &mut Buffer, x: u16, stil: Style| {
        buf.set_stringn(x, y, "─".repeat(bredde.saturating_sub(x) as usize), b, stil);
    };
    if stripe {
        strek(buf, 0, SVAK);
        return true;
    }
    // Uten farge, for i en gruppe er det Claude-er i flere tilstander.
    let x = buf.set_stringn(1, y, o.tittel(), b, STERK).0 + 1;
    // Boksene har kanter nok, så mappene har bare en strek uten dem. Aktive og
    // ledige har alltid strek, så skillet mellom dem synes.
    if u.plass != Plass::Minimal && matches!(o, Overskrift::Mappe(..)) {
        return false;
    }
    strek(buf, x, SVAK);
    true
}

/// Linjene i kortet, der `bredde` er lista uten skillelinja. Minimal er én
/// linje uten boks.
pub fn kort(u: Utseende, i: &Info, bredde: u16, valgt: bool, stripe: bool) -> Vec<Linje> {
    if stripe {
        return vec![vec![peker(valgt), bit(1, glyf(i.merke), stripestil(u, i.merke))]];
    }
    // Teksten går fram til en kolonne før skillelinja.
    let slutt = bredde.saturating_sub(1);
    match u.plass {
        Plass::Minimal => linje(u, i, slutt, valgt),
        Plass::Vanlig => boks(u, i, slutt, valgt),
    }
}

fn farge(gruppe: Gruppe) -> Color {
    match gruppe {
        Gruppe::Venter => ORANSJE,
        Gruppe::Jobber => GRØNN,
        Gruppe::Ferdig => Color::Reset,
    }
}

fn tone(gruppe: Gruppe) -> Option<Color> {
    match gruppe {
        Gruppe::Venter => tonet(Some(ORANSJE_RGB), TONE),
        Gruppe::Jobber => tonet(Some(GRØNN_RGB), TONE),
        Gruppe::Ferdig => None,
    }
}

fn glyfstil(u: Utseende, merke: Merke) -> Style {
    match (u.farge, merke.gruppe()) {
        (_, Gruppe::Ferdig) => SVAK,
        (Farge::Av, _) => VANLIG,
        (Farge::På, g) => Style::new().fg(farge(g)),
    }
}

/// På stripa er tegnene alt som står, så de ledige er ikke dempet.
fn stripestil(u: Utseende, merke: Merke) -> Style {
    match u.farge {
        Farge::Av => VANLIG,
        Farge::På => Style::new().fg(farge(merke.gruppe())),
    }
}

fn tidstil(u: Utseende, merke: Merke) -> Style {
    match (u.farge, merke.gruppe()) {
        (Farge::Av, _) | (_, Gruppe::Ferdig) => SVAK,
        (Farge::På, g) => Style::new().fg(farge(g)),
    }
}

/// Prosjektet i toppkanten. Uten oppgave er prosjektet tittelen, og gruppert
/// etter mappe står det i overskriften.
fn prosjekt(i: &Info) -> Option<String> {
    Some(i.sted.clone()).filter(|_| !i.sted_over && !i.starter && i.oppgave.is_some())
}

fn tittel_på(i: &Info) -> &str {
    i.oppgave.as_deref().filter(|_| !i.starter).unwrap_or(&i.sted)
}

/// Glyfen, tittelen og tiden på én linje.
fn linje(u: Utseende, i: &Info, slutt: u16, valgt: bool) -> Vec<Linje> {
    let tid = i.tid.as_deref();
    let plass = (slutt.saturating_sub(ETTER_GLYF) as usize).saturating_sub(tid.map_or(0, |t| t.chars().count() + 2));
    // På én linje vises mest av tittelen når den kuttes midt i et ord.
    let mut ut = vec![peker(valgt), bit(INNRYKK, glyf(i.merke), glyfstil(u, i.merke)), bit(ETTER_GLYF, kutt(tittel_på(i), plass), VANLIG)];
    if let Some(tid) = tid {
        ut.push(bit(slutt.saturating_sub(bredde(tid)), tid, tidstil(u, i.merke)));
    }
    vec![ut]
}

/// Med farge har rammen en tone av køens farge, og den valgte får hele
/// fargen.
fn rammestil(u: Utseende, merke: Merke, valgt: bool) -> Style {
    let gruppe = merke.gruppe();
    match (u.farge, valgt, tone(gruppe)) {
        (Farge::På, true, Some(_)) => Style::new().fg(farge(gruppe)),
        (Farge::På, false, Some(tone)) => Style::new().fg(tone),
        (_, true, _) => VANLIG,
        _ => SVAK,
    }
}

/// Prosjektet og glyfen i toppkanten med tiden til høyre, tittelen inni og
/// branchen under.
fn boks(u: Utseende, i: &Info, b: u16, valgt: bool) -> Vec<Linje> {
    let ramme = rammestil(u, i.merke, valgt);
    let indre = b.saturating_sub(2 * INNRYKK) as usize;
    let glyf = (glyf(i.merke).to_string(), glyfstil(u, i.merke));
    let tid = i.tid.clone().map(|t| (t, tidstil(u, i.merke)));
    // To mellomrom, så glyfen ikke treffer prosjektet når fonten tegner den
    // over to kolonner.
    let venstre: Kanttekst = [glyf].into_iter().chain(prosjekt(i).map(|p| (format!("  {p}"), STERK))).collect();
    let høyre: Kanttekst = tid.into_iter().collect();
    let mut inni: Vec<(String, Style)> = match u.tittel {
        // På én linje vises mest av tittelen når den kuttes midt i et ord.
        1 => vec![kutt(tittel_på(i), indre)],
        n => bryt(tittel_på(i), indre, indre, n),
    }
    .into_iter()
    .map(|l| (l, VANLIG))
    .collect();
    match (i.starter, &i.branch) {
        (true, _) => inni.push(("starter …".to_string(), SVAK)),
        (false, Some(b)) => inni.push((kutt(b, indre), SVAK)),
        (false, None) => {}
    }
    let mut ut = vec![kantlinje(b, ("╭", "╮"), venstre, høyre, ramme)];
    for (tekst, stil) in inni {
        ut.push(vec![bit(0, "│", ramme), bit(INNRYKK, tekst, stil), bit(b.saturating_sub(1), "│", ramme)]);
    }
    ut.push(kantlinje(b, ("╰", "╯"), Vec::new(), Vec::new(), ramme));
    ut
}

/// Øverste eller nederste kant i en boks `b` bred, med tekst til venstre og
/// høyre i streken.
fn kantlinje(b: u16, (v, h): (&str, &str), venstre: Vec<(String, Style)>, høyre: Vec<(String, Style)>, ramme: Style) -> Linje {
    let mut l = vec![bit(0, format!("{v}─"), ramme)];
    let mut x = 2;
    let høyre_bredde = if høyre.is_empty() { 0 } else { høyre.iter().map(|(t, _)| bredde(t)).sum::<u16>() + 2 };
    let slutt = b.saturating_sub(2 + høyre_bredde);
    let med_luft = |l: &mut Linje, x: &mut u16, biter: Vec<(String, Style)>, plass: Option<u16>| {
        l.push(bit(*x, " ", ramme));
        *x += 1;
        for (t, s) in biter {
            let t = match plass {
                Some(slutt) => kutt(&t, slutt.saturating_sub(*x + 1) as usize),
                None => t,
            };
            let w = bredde(&t);
            l.push(bit(*x, t, s));
            *x += w;
        }
        l.push(bit(*x, " ", ramme));
        *x += 1;
    };
    if !venstre.is_empty() {
        med_luft(&mut l, &mut x, venstre, Some(slutt));
    }
    if slutt > x {
        l.push(bit(x, "─".repeat((slutt - x) as usize), ramme));
        x = slutt;
    }
    if !høyre.is_empty() {
        med_luft(&mut l, &mut x, høyre, None);
    }
    l.push(bit(x, format!("─{h}"), ramme));
    l
}

#[cfg(test)]
mod tester {
    use super::*;
    use crate::bokser::Rammer;

    fn info(oppgave: Option<&str>, branch: Option<&str>) -> Info {
        Info {
            merke: Merke::Jobber,
            oppgave: oppgave.map(str::to_string),
            sted: "nerdr".into(),
            branch: branch.map(str::to_string),
            tid: Some("3m".into()),
            starter: false,
            sted_over: false,
        }
    }

    /// Bitene lagt over hverandre, slik de tegnes.
    fn tekst(linje: &Linje) -> String {
        let mut ut: Vec<char> = Vec::new();
        for bit in linje {
            for (i, c) in bit.tekst.chars().enumerate() {
                let x = bit.x as usize + i;
                if ut.len() <= x {
                    ut.resize(x + 1, ' ');
                }
                ut[x] = c;
            }
        }
        ut.into_iter().collect::<String>().trim_end().to_string()
    }

    fn linjer(u: Utseende, i: &Info, bredde: u16) -> Vec<String> {
        kort(u, i, bredde, false, false).iter().map(tekst).collect()
    }

    #[test]
    fn minimal_er_glyfen_tittelen_og_tiden_på_én_linje() {
        let u = Utseende { plass: Plass::Minimal, ..Utseende::ny(Rammer::Linjer) };
        assert_eq!(linjer(u, &info(Some("Fiks testene"), Some("main")), 30), ["  ◐  Fiks testene          3m"]);
    }

    #[test]
    fn glyfen_og_prosjektet_i_toppkanten_og_branchen_under() {
        let u = Utseende::ny(Rammer::Linjer);
        assert_eq!(
            linjer(u, &info(Some("Fiks testene"), Some("main")), 24),
            ["╭─ ◐  nerdr ───── 3m ─╮", "│ Fiks testene        │", "│ main                │", "╰─────────────────────╯"]
        );
    }

    #[test]
    fn uten_branch() {
        let u = Utseende::ny(Rammer::Linjer);
        assert_eq!(linjer(u, &info(Some("Fiks testene"), None), 24), ["╭─ ◐  nerdr ───── 3m ─╮", "│ Fiks testene        │", "╰─────────────────────╯"]);
    }

    #[test]
    fn tittelen_på_så_mange_linjer_som_valgt() {
        let i = info(Some("Fiks testene som feiler i CI"), None);
        let u = Utseende::ny(Rammer::Linjer);
        assert_eq!(linjer(u, &i, 20)[1], "│ Fiks testene s… │");
        assert_eq!(linjer(Utseende { tittel: 2, ..u }, &i, 20)[1..3], ["│ Fiks testene    │", "│ som feiler i CI │"]);
    }
}
