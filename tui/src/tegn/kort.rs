//! Kortene i lista og overskriftene over gruppene, etter utseendet. Et kort er
//! linjer med tekstbiter, så høyden er kjent før det tegnes, og et klikk kan
//! treffe riktig kort.

use nerdr_kjerne::merke::{Gruppe, Merke};
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier, Style};

use super::{GRØNN, GRØNN_RGB, ORANSJE, ORANSJE_RGB, STERK, SVAK, VANLIG, bryt, kutt, tonet};
use crate::grupper::{Rad, glyf, tittel};
use crate::utseende::{Farge, Liste, Plass, Utseende};

/// I originalen starter teksten etter peker, glyf og to mellomrom. Glyfene
/// har tvetydig bredde i Unicode, og mange fonter tegner dem over to kolonner.
const TEKST_X: u16 = 4;
/// Uten glyf foran starter teksten etter pekeren og et mellomrom.
const INNRYKK: u16 = 2;
/// Tonen av køens farge i rammer og streker med mye farge.
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
}

pub struct Bit {
    pub x: u16,
    pub tekst: String,
    pub stil: Style,
}

pub type Linje = Vec<Bit>;

fn bit(x: u16, tekst: impl Into<String>, stil: Style) -> Bit {
    Bit { x, tekst: tekst.into(), stil }
}

fn bredde(tekst: &str) -> u16 {
    tekst.chars().count() as u16
}

fn peker(valgt: bool) -> Bit {
    bit(0, if valgt { "▎" } else { " " }, VANLIG)
}

fn ny(u: Utseende, stripe: bool) -> bool {
    !stripe && u.liste != Liste::Original
}

/// Lista starter under streken øverst. Boksene står tett, og uten bokser er
/// det luft mellom kortene, unntatt når det er tett. Stripa har ingen streker.
pub fn ordne(u: Utseende, rader: Vec<Rad<'_>>, stripe: bool) -> Vec<Rad<'_>> {
    let ny = ny(u, stripe);
    let mut ut = Vec::new();
    if ny {
        ut.push(Rad::Tom);
    }
    for rad in rader {
        match rad {
            Rad::Strek if stripe || ny && (u.liste == Liste::Bokser || u.plass == Plass::Tett) => continue,
            Rad::Tom if ny && u.plass == Plass::Romslig => ut.push(Rad::Tom),
            _ => {}
        }
        ut.push(rad);
    }
    ut
}

pub fn antall<'a, 'b: 'a>(rader: impl IntoIterator<Item = &'a Rad<'b>>) -> Vec<(Gruppe, usize)> {
    let mut ut: Vec<(Gruppe, usize)> = Vec::new();
    for rad in rader {
        match rad {
            Rad::Overskrift(g) => ut.push((*g, 0)),
            Rad::Pane(..) => {
                if let Some(a) = ut.last_mut() {
                    a.1 += 1;
                }
            }
            _ => {}
        }
    }
    ut
}

/// Overskriften over en gruppe, der `bredde` er lista uten skillelinja. Gir
/// om streken går til skillelinja.
pub fn overskrift(u: Utseende, buf: &mut Buffer, y: u16, bredde: u16, gruppe: Gruppe, antall: &[(Gruppe, usize)], stripe: bool) -> bool {
    let b = bredde as usize;
    let strek = |buf: &mut Buffer, x: u16, stil: Style| {
        buf.set_stringn(x, y, "─".repeat(bredde.saturating_sub(x) as usize), b, stil);
    };
    if stripe {
        strek(buf, 0, SVAK);
        return true;
    }
    if u.liste == Liste::Original {
        let x = buf.set_stringn(1, y, tittel(gruppe), b, SVAK).0 + 1;
        strek(buf, x, SVAK);
        return true;
    }
    let n = antall.iter().find(|a| a.0 == gruppe).map_or(0, |a| a.1);
    let stil = match (u.farge, gruppe) {
        (Farge::Lite, _) | (_, Gruppe::Ferdig) => STERK,
        _ => Style::new().fg(farge(gruppe)).add_modifier(Modifier::BOLD),
    };
    let x = buf.set_stringn(1, y, tittel(gruppe), b, stil).0;
    let x = buf.set_stringn(x + 1, y, n.to_string(), b, SVAK).0 + 1;
    if u.liste == Liste::Bokser {
        return false;
    }
    let stil = match u.farge {
        Farge::Mye => tone(gruppe).map_or(SVAK, |c| Style::new().fg(c)),
        _ => SVAK,
    };
    strek(buf, x, stil);
    true
}

/// Streken mellom to kort i samme gruppe. Bare originalen har en.
pub fn strek(u: Utseende, buf: &mut Buffer, y: u16, bredde: u16) {
    if u.liste == Liste::Original {
        let lengde = bredde.saturating_sub(TEKST_X + 1) as usize;
        buf.set_stringn(TEKST_X, y, "┈".repeat(lengde), lengde, SVAK);
    }
}

/// Linjene i kortet, der `bredde` er lista uten skillelinja.
pub fn kort(u: Utseende, i: &Info, bredde: u16, valgt: bool, stripe: bool) -> Vec<Linje> {
    if stripe {
        return vec![vec![peker(valgt), bit(1, glyf(i.merke), Style::new().fg(glyffarge(i.merke)))]];
    }
    // Teksten går fram til en kolonne før skillelinja.
    let slutt = bredde.saturating_sub(1);
    match u.liste {
        Liste::Original => original(i, slutt, valgt),
        Liste::Bokser => boks(u, i, slutt, valgt),
        Liste::Linjer => linje(u, i, slutt, valgt),
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

fn glyffarge(merke: Merke) -> Color {
    match merke {
        Merke::Spør | Merke::Ulest => ORANSJE,
        Merke::Jobber | Merke::Starter => GRØNN,
        Merke::Sett => Color::Reset,
    }
}

fn glyfstil(merke: Merke) -> Style {
    match merke.gruppe() {
        Gruppe::Ferdig => SVAK,
        g => Style::new().fg(farge(g)),
    }
}

fn tidstil(u: Utseende, merke: Merke) -> Style {
    match (u.farge, merke.gruppe()) {
        (Farge::Lite, _) | (_, Gruppe::Ferdig) | (Farge::Middels, Gruppe::Jobber) => SVAK,
        (_, g) => Style::new().fg(farge(g)),
    }
}

/// Fet tittel i alle køene, så den leses først. Ferdige er vanlige, ikke svake.
fn tittelstil(merke: Merke, valgt: bool) -> Style {
    match (valgt, merke.gruppe()) {
        (false, Gruppe::Ferdig) => VANLIG,
        _ => STERK,
    }
}

fn tittel_på(i: &Info) -> &str {
    i.oppgave.as_deref().filter(|_| !i.starter).unwrap_or(&i.sted)
}

/// Repoet og branchen under tittelen. Uten oppgave er repoet tittelen, og
/// romslig har repoet og branchen på hver sin linje.
fn under(u: Utseende, i: &Info, bredde: usize) -> Vec<String> {
    let linjer = match (i.starter, &i.oppgave, &i.branch) {
        (true, _, _) => vec!["starter …".to_string()],
        (false, Some(_), Some(b)) if u.plass == Plass::Romslig => vec![i.sted.clone(), format!("› {b}")],
        (false, Some(_), Some(b)) => vec![format!("{} › {b}", i.sted)],
        (false, Some(_), None) => vec![i.sted.clone()],
        (false, None, Some(b)) => vec![format!("› {b}")],
        (false, None, None) => Vec::new(),
    };
    linjer.iter().map(|l| kutt(l, bredde)).collect()
}

/// Glyfen foran, oppgaven som tittel med tiden til høyre, og repoet og
/// branchen under.
fn original(i: &Info, slutt: u16, valgt: bool) -> Vec<Linje> {
    let tekst = slutt.saturating_sub(TEKST_X) as usize;
    let første = tekst.saturating_sub(i.tid.as_ref().map_or(0, |t| t.chars().count() + 2));
    let oppgave = i.oppgave.as_deref().filter(|_| !i.starter).map(|o| bryt(o, første, tekst, 2)).filter(|l| !l.is_empty());
    let (tittel, under) = match (i.starter, oppgave) {
        (true, _) => (vec![kutt(&i.sted, første)], vec!["starter …".to_string()]),
        (false, Some(oppgave)) => (oppgave, repo_og_branch(&i.sted, i.branch.as_deref(), tekst)),
        (false, None) => (vec![kutt(&i.sted, første)], i.branch.iter().map(|b| kutt(&format!("› {b}"), tekst)).collect()),
    };
    let tittelstil = match (valgt, i.merke.gruppe()) {
        (true, _) => STERK,
        (false, Gruppe::Ferdig) => SVAK,
        (false, _) => VANLIG,
    };
    let mut ut: Vec<Linje> = tittel.into_iter().map(|l| vec![peker(valgt), bit(TEKST_X, l, tittelstil)]).collect();
    ut.extend(under.into_iter().map(|l| vec![peker(valgt), bit(TEKST_X, l, SVAK)]));
    ut[0].push(bit(1, glyf(i.merke), Style::new().fg(glyffarge(i.merke))));
    if let Some(tid) = &i.tid {
        ut[0].push(bit(slutt.saturating_sub(bredde(tid)), tid, SVAK));
    }
    ut
}

/// På én linje når begge får plass, ellers branchen på linja under, så ingen
/// av dem kuttes før de må.
fn repo_og_branch(sted: &str, branch: Option<&str>, bredde: usize) -> Vec<String> {
    let Some(branch) = branch else { return vec![kutt(sted, bredde)] };
    let sammen = format!("{sted} › {branch}");
    if sammen.chars().count() <= bredde {
        return vec![sammen];
    }
    vec![kutt(sted, bredde), kutt(&format!("› {branch}"), bredde)]
}

/// Uten kort: tittelen helt til venstre, tiden og glyfen til høyre, og repoet
/// dempet under. Tett er bare tittelen, på én linje.
fn linje(u: Utseende, i: &Info, slutt: u16, valgt: bool) -> Vec<Linje> {
    let tekst = slutt.saturating_sub(INNRYKK) as usize;
    let tid = i.tid.as_deref().filter(|_| u.tid);
    let første = tekst.saturating_sub(tid.map_or(2, |t| t.chars().count() + 3) + 1);
    let tittel = tittel_på(i);
    let linjer = match u.plass {
        // På én linje vises mest av tittelen når den kuttes midt i et ord.
        Plass::Tett => vec![kutt(tittel, første)],
        Plass::Vanlig => bryt(tittel, første, tekst, 2),
        Plass::Romslig => bryt(tittel, første, tekst, 3),
    };
    let mut ut: Vec<Linje> = linjer.into_iter().map(|l| vec![peker(valgt), bit(INNRYKK, l, tittelstil(i.merke, valgt))]).collect();
    if ut.is_empty() {
        ut.push(vec![peker(valgt)]);
    }
    if u.plass != Plass::Tett {
        ut.extend(under(u, i, tekst).into_iter().map(|l| vec![peker(valgt), bit(INNRYKK, l, SVAK)]));
    }
    ut[0].push(bit(slutt.saturating_sub(1), glyf(i.merke), glyfstil(i.merke)));
    if let Some(tid) = tid {
        ut[0].push(bit(slutt.saturating_sub(2 + bredde(tid)), tid, tidstil(u, i.merke)));
    }
    ut
}

/// Med mye farge har rammen en tone av køens farge, og den valgte får hele
/// fargen.
fn rammestil(u: Utseende, merke: Merke, valgt: bool) -> Style {
    let gruppe = merke.gruppe();
    match (u.farge, valgt, tone(gruppe)) {
        (Farge::Mye, true, Some(_)) => Style::new().fg(farge(gruppe)),
        (Farge::Mye, false, Some(tone)) => Style::new().fg(tone),
        (_, true, _) => VANLIG,
        _ => SVAK,
    }
}

/// Glyfen og tiden i toppkanten og tittelen inni. Uten tid står glyfen til
/// høyre, der tiden ellers står. Tett har repoet i bunnkanten, og romslig gir
/// tittelen tre linjer.
fn boks(u: Utseende, i: &Info, b: u16, valgt: bool) -> Vec<Linje> {
    let ramme = rammestil(u, i.merke, valgt);
    let indre = b.saturating_sub(2 * INNRYKK) as usize;
    let glyf = vec![(glyf(i.merke).to_string(), glyfstil(i.merke))];
    let topp = match i.tid.clone().filter(|_| u.tid) {
        Some(tid) => kantlinje(b, ("╭", "╮"), glyf, vec![(tid, tidstil(u, i.merke))], ramme),
        None => kantlinje(b, ("╭", "╮"), Vec::new(), glyf, ramme),
    };
    let maks = if u.plass == Plass::Romslig { 3 } else { 2 };
    let mut inni: Vec<(String, Style)> = bryt(tittel_på(i), indre, indre, maks).into_iter().map(|l| (l, tittelstil(i.merke, valgt))).collect();
    let tett = u.plass == Plass::Tett;
    if !tett {
        inni.extend(under(u, i, indre).into_iter().map(|l| (l, SVAK)));
    }
    let mut ut = vec![topp];
    for (tekst, stil) in inni {
        ut.push(vec![bit(0, "│", ramme), bit(INNRYKK, tekst, stil), bit(b.saturating_sub(1), "│", ramme)]);
    }
    let bunn = if tett { under(u, i, indre).into_iter().map(|l| (l, SVAK)).collect() } else { Vec::new() };
    ut.push(kantlinje(b, ("╰", "╯"), bunn, Vec::new(), ramme));
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

    #[test]
    fn repo_og_branch_på_én_linje_når_det_er_plass() {
        assert_eq!(repo_og_branch("nerdr", Some("main"), 30), ["nerdr › main"]);
        assert_eq!(repo_og_branch("nerdr", None, 30), ["nerdr"]);
        assert_eq!(
            repo_og_branch("api-bm-customer", Some("legg-til-audit-kode"), 30),
            ["api-bm-customer", "› legg-til-audit-kode"]
        );
        assert_eq!(repo_og_branch("api-bm-customer", Some("legg-til-audit-kode"), 40), ["api-bm-customer › legg-til-audit-kode"]);
    }

    #[test]
    fn linjer_har_tiden_og_glyfen_til_høyre() {
        let u = Utseende::ny(Rammer::Linjer);
        let kort = kort(u, &info(Some("Fiks testene"), Some("main")), 30, false, false);
        assert_eq!(kort.iter().map(tekst).collect::<Vec<_>>(), ["  Fiks testene           3m ◐", "  nerdr › main"]);
    }

    #[test]
    fn tett_er_én_linje() {
        let u = Utseende { plass: Plass::Tett, ..Utseende::ny(Rammer::Linjer) };
        assert_eq!(kort(u, &info(Some("Fiks testene"), Some("main")), 30, false, false).len(), 1);
    }

    #[test]
    fn boksen_har_glyfen_til_høyre_uten_tid() {
        let u = Utseende { liste: Liste::Bokser, tid: false, ..Utseende::ny(Rammer::Bokser) };
        let kort = kort(u, &info(Some("Fiks testene"), None), 20, false, false);
        assert_eq!(kort.iter().map(tekst).collect::<Vec<_>>(), ["╭───────────── ◐ ─╮", "│ Fiks testene    │", "│ nerdr           │", "╰─────────────────╯"]);
    }

    #[test]
    fn romslig_har_repoet_og_branchen_på_hver_sin_linje() {
        let u = Utseende { plass: Plass::Romslig, ..Utseende::ny(Rammer::Linjer) };
        assert_eq!(under(u, &info(Some("x"), Some("main")), 30), ["nerdr", "› main"]);
    }
}
