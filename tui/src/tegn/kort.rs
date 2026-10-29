//! Kortene i lista og overskriftene over gruppene, etter utseendet. Et kort er
//! linjer med tekstbiter, så høyden er kjent før det tegnes, og et klikk kan
//! treffe riktig kort.

use nerdr_kjerne::merke::{Gruppe, Merke};
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Style};

use super::{BLÅ, BLÅ_RGB, GRØNN, GRØNN_RGB, ORANSJE, ORANSJE_RGB, STERK, SVAK, VANLIG, bryt, kutt, kutt_foran, tonet};
use crate::palett::Rgb;
use crate::grupper::{Overskrift, Rad, glyf};
use crate::utseende::{Farge, Kort, Plass, Sti, Topp, Ulestfarge, Utseende, Valgt};

/// Uten glyf foran starter teksten etter pekeren og et mellomrom.
const INNRYKK: u16 = 2;
/// Teksten etter glyfen i minimal. Glyfene har tvetydig bredde i Unicode, og
/// mange fonter tegner dem over to kolonner.
const ETTER_GLYF: u16 = INNRYKK + 3;
/// Tonen av køens farge i rammer og streker.
const TONE: f32 = 0.5;
/// Navnet etter trekanten, med to kolonner mellom, fordi mange fonter tegner
/// trekanten over to.
const ETTER_PIL: u16 = 4;

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
    let mut x = 1;
    if matches!(o, Overskrift::Mappe(..)) {
        buf.set_stringn(x, y, "▾", b, SVAK);
        x = ETTER_PIL;
    }
    let x = buf.set_stringn(x, y, o.tittel(), b, STERK).0 + 1;
    // Boksene har kanter nok, så mappene har bare en strek uten dem. Aktive og
    // ledige har alltid strek, så skillet mellom dem synes.
    if u.plass != Plass::Minimal && matches!(o, Overskrift::Mappe(..)) {
        return false;
    }
    strek(buf, x, SVAK);
    true
}

/// Et prosjekt brettet sammen til én linje, med tegnet til hver Claude etter
/// navnet. På stripa står bare tegnet til den som haster mest.
pub fn brettet(u: Utseende, navn: &str, merker: &[Merke], bredde: u16, valgt: bool, stripe: bool) -> Linje {
    let haster = merker.iter().copied().min_by_key(|m| m.gruppe()).unwrap_or(Merke::Sett);
    if stripe {
        return vec![peker(valgt), bit(1, glyf(haster), stripestil(u, haster))];
    }
    let slutt = bredde.saturating_sub(1);
    let navn = kutt(navn, slutt.saturating_sub(10) as usize);
    let mut x = ETTER_PIL + self::bredde(&navn) + 2;
    let mut linje = vec![peker(valgt), bit(1, "▸", SVAK), bit(ETTER_PIL, navn, STERK)];
    // Tre kolonner for hvert, fordi mange fonter tegner glyfene over to.
    for merke in merker {
        if x + 2 > slutt {
            linje.push(bit(x, "…", SVAK));
            break;
        }
        linje.push(bit(x, glyf(*merke), glyfstil(u, *merke)));
        x += 3;
    }
    linje
}

/// Linjene i kortet, der `bredde` er lista uten skillelinja. Minimal er én
/// linje uten boks.
pub fn kort(u: Utseende, i: &Info, bredde: u16, valgt: bool, stripe: bool) -> Vec<Linje> {
    if stripe {
        return vec![vec![peker(valgt), bit(1, glyf(i.merke), stripestil(u, i.merke))]];
    }
    // Teksten går fram til en kolonne før skillelinja.
    let slutt = bredde.saturating_sub(1);
    match (u.plass, u.form()) {
        (Plass::Minimal, _) => linje(u, i, slutt, valgt),
        (_, Kort::Kant) => kant(u, i, slutt),
        _ => boks(u, i, slutt, valgt),
    }
}

/// Glyfen, tittelen og tiden på første linje og branchen under, med teksten
/// fra `tekst_x` og tiden fram til `slutt`.
fn innhold(u: Utseende, i: &Info, glyf_x: Option<u16>, tekst_x: u16, slutt: u16) -> Vec<Linje> {
    let tid = i.tid.as_deref();
    let plass = slutt.saturating_sub(tekst_x + tid.map_or(0, |t| bredde(t) + 2)) as usize;
    let titler = match u.tittel {
        1 => vec![kutt(tittel_på(i), plass)],
        n => bryt(tittel_på(i), plass, plass, n),
    };
    let mut ut: Vec<Linje> = Vec::new();
    for (n, tittel) in titler.into_iter().enumerate() {
        let mut l = vec![bit(tekst_x, tittel, VANLIG)];
        if n == 0 {
            if let Some(x) = glyf_x {
                l.push(bit(x, glyf(i.merke), glyfstil(u, i.merke)));
            }
            if let Some(tid) = tid {
                l.push(bit(slutt.saturating_sub(bredde(tid)), tid, tidstil(u, i.merke)));
            }
        }
        ut.push(l);
    }
    match (i.starter, &i.branch) {
        (true, _) => ut.push(vec![bit(tekst_x, "starter …", SVAK)]),
        (false, Some(b)) => ut.push(vec![bit(tekst_x, kutt(b, slutt.saturating_sub(tekst_x) as usize), SVAK)]),
        (false, None) => {}
    }
    ut
}

/// Fargen i en kant til venstre, og ingen boks.
fn kant(u: Utseende, i: &Info, slutt: u16) -> Vec<Linje> {
    let stil = glyfstil(u, i.merke);
    innhold(u, i, None, INNRYKK + 1, slutt)
        .into_iter()
        .map(|mut l| {
            l.insert(0, bit(1, "▌", stil));
            l
        })
        .collect()
}

/// En Claude i rammen rundt prosjektet, med sidene i `ramme`.
pub fn seksjon(u: Utseende, i: &Info, bredde: u16, valgt: bool, ramme: Style) -> Vec<Linje> {
    let b = bredde.saturating_sub(1);
    let (side, stil) = if valgt && u.valgt == Valgt::Tykk { ("┃", full(u, i.merke)) } else { ("│", ramme) };
    innhold(u, i, Some(INNRYKK), ETTER_GLYF, b.saturating_sub(2))
        .into_iter()
        .map(|mut l| {
            l.insert(0, bit(0, side, stil));
            l.push(bit(b.saturating_sub(1), side, stil));
            l
        })
        .collect()
}

/// Hele fargen til tilstanden, til streker som skal synes.
fn full(u: Utseende, merke: Merke) -> Style {
    match (u.farge, merke.gruppe()) {
        (Farge::På, Gruppe::Venter | Gruppe::Jobber) => Style::new().fg(farge(u, merke)),
        _ => VANLIG,
    }
}

fn haster(merker: &[Merke]) -> Merke {
    merker.iter().copied().min_by_key(|m| (m.gruppe(), *m != Merke::Spør)).unwrap_or(Merke::Sett)
}

/// Rammen rundt et prosjekt har tonen til den som haster mest.
pub fn prosjektramme(u: Utseende, merker: &[Merke]) -> Style {
    rammestil(u, haster(merker), false)
}

/// Tegnet til hver Claude, med to mellomrom mellom, fordi mange fonter tegner
/// glyfene over to kolonner.
fn glyfer(u: Utseende, merker: &[Merke]) -> Kanttekst {
    let mut ut = Vec::new();
    for (n, merke) in merker.iter().enumerate() {
        if n > 0 {
            ut.push(("  ".to_string(), VANLIG));
        }
        ut.push((glyf(*merke).to_string(), glyfstil(u, *merke)));
    }
    ut.push((" ".to_string(), VANLIG));
    ut
}

/// Toppen av rammen rundt et prosjekt, med navnet og tegnene. Brettet er
/// toppen og bunnen.
pub fn ramme_topp(u: Utseende, navn: &str, merker: &[Merke], bredde: u16, brettet: bool) -> Vec<Linje> {
    let b = bredde.saturating_sub(1);
    let stil = prosjektramme(u, merker);
    let pil = if brettet { "▸  " } else { "▾  " };
    let venstre = vec![(pil.to_string(), SVAK), (navn.to_string(), STERK)];
    let høyre = if brettet { glyfer(u, merker) } else { Vec::new() };
    let mut ut = vec![kantlinje(b, ("╭", "╮"), "─", venstre, høyre, stil)];
    if brettet {
        ut.push(ramme_bunn(u, merker, bredde));
    }
    ut
}

pub fn ramme_skille(u: Utseende, merker: &[Merke], bredde: u16) -> Linje {
    kantlinje(bredde.saturating_sub(1), ("├", "┤"), "─", Vec::new(), Vec::new(), prosjektramme(u, merker))
}

pub fn ramme_bunn(u: Utseende, merker: &[Merke], bredde: u16) -> Linje {
    kantlinje(bredde.saturating_sub(1), ("╰", "╯"), "─", Vec::new(), Vec::new(), prosjektramme(u, merker))
}

/// Prosjektet på en rad for seg, med stien til repoet etter valget. Brettet
/// står tegnet til hver Claude til høyre, og ellers står de på kortene under.
pub fn overskriftrad(
    u: Utseende,
    navn: &str,
    sti: Option<&str>,
    merker: &[Merke],
    bredde: u16,
    brettet: bool,
    valgt: bool,
) -> Vec<Linje> {
    let merker = if brettet { merker } else { &[] };
    let slutt = bredde.saturating_sub(2);
    let start = slutt.saturating_sub(merker.len() as u16 * 3);
    let plass = start.saturating_sub(ETTER_PIL + 1) as usize;
    let pil = if brettet { "▸" } else { "▾" };
    let forelder = sti.map(|s| s.rsplit_once('/').map_or(s, |(f, _)| f));
    let mut l = vec![peker(valgt), bit(1, pil, SVAK)];
    let mut x = ETTER_PIL;
    let mut tekst = |l: &mut Linje, t: String, stil: Style| {
        let w = self::bredde(&t);
        l.push(bit(x, t, stil));
        x += w;
    };
    match (u.sti, forelder) {
        (Sti::Før, Some(f)) => {
            let navn = kutt(navn, plass);
            let foran = kutt_foran(&format!("{f}/"), plass.saturating_sub(self::bredde(&navn) as usize));
            tekst(&mut l, foran, SVAK);
            tekst(&mut l, navn, STERK);
        }
        (Sti::Etter, Some(f)) => {
            let navn = kutt(navn, plass);
            let etter = plass.saturating_sub(self::bredde(&navn) as usize + 2);
            tekst(&mut l, navn, STERK);
            if etter > 3 {
                tekst(&mut l, format!("  {}", kutt_foran(f, etter)), SVAK);
            }
        }
        _ => tekst(&mut l, kutt(navn, plass), STERK),
    }
    let etter_navn = x + 1;
    if u.topp == Topp::Strek {
        let til = if merker.is_empty() { bredde } else { start };
        l.push(bit(etter_navn, "─".repeat(til.saturating_sub(etter_navn) as usize), SVAK));
    }
    for (n, merke) in merker.iter().enumerate() {
        l.push(bit(start + 1 + n as u16 * 3, glyf(*merke), glyfstil(u, *merke)));
    }
    let mut ut = vec![l];
    if let (Sti::Under, Some(sti)) = (u.sti, sti) {
        ut.push(vec![bit(ETTER_PIL, kutt_foran(sti, slutt.saturating_sub(ETTER_PIL) as usize), SVAK)]);
    }
    ut
}

/// Fargen til tilstanden, og den samme i RGB til tonene. De ledige har ingen.
fn rgb(u: Utseende, merke: Merke) -> Option<(Color, Rgb)> {
    match merke.gruppe() {
        Gruppe::Venter if merke == Merke::Ulest && u.ulest == Ulestfarge::Blå => Some((BLÅ, BLÅ_RGB)),
        Gruppe::Venter => Some((ORANSJE, ORANSJE_RGB)),
        Gruppe::Jobber => Some((GRØNN, GRØNN_RGB)),
        Gruppe::Ferdig => None,
    }
}

fn farge(u: Utseende, merke: Merke) -> Color {
    rgb(u, merke).map_or(Color::Reset, |(c, _)| c)
}

fn tone(u: Utseende, merke: Merke) -> Option<Color> {
    rgb(u, merke).and_then(|(_, r)| tonet(Some(r), TONE))
}

fn glyfstil(u: Utseende, merke: Merke) -> Style {
    match (u.farge, merke.gruppe()) {
        (_, Gruppe::Ferdig) => SVAK,
        (Farge::Av, _) => VANLIG,
        (Farge::På, _) => Style::new().fg(farge(u, merke)),
    }
}

/// På stripa er tegnene alt som står, så de ledige er ikke dempet.
fn stripestil(u: Utseende, merke: Merke) -> Style {
    match u.farge {
        Farge::Av => VANLIG,
        Farge::På => Style::new().fg(farge(u, merke)),
    }
}

fn tidstil(u: Utseende, merke: Merke) -> Style {
    match (u.farge, merke.gruppe()) {
        (Farge::Av, _) | (_, Gruppe::Ferdig) => SVAK,
        (Farge::På, _) => Style::new().fg(farge(u, merke)),
    }
}

/// Prosjektet i toppkanten. Uten oppgave er prosjektet tittelen, og gruppert
/// etter mappe står det i overskriften.
fn prosjekt(i: &Info) -> Option<String> {
    (!i.sted_over && !i.starter && i.oppgave.is_some()).then_some(i.sted.clone())
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
    match (u.farge, valgt, tone(u, merke)) {
        (Farge::På, true, Some(_)) => Style::new().fg(farge(u, merke)),
        (Farge::På, false, Some(tone)) => Style::new().fg(tone),
        (_, true, _) => VANLIG,
        _ => SVAK,
    }
}

/// Prosjektet og glyfen i toppkanten med tiden til høyre, tittelen inni og
/// branchen under.
fn boks(u: Utseende, i: &Info, b: u16, valgt: bool) -> Vec<Linje> {
    let tykk = valgt && u.valgt == Valgt::Tykk;
    let ramme = if tykk { full(u, i.merke) } else { rammestil(u, i.merke, valgt) };
    let (topp, bunn, strek, side) = if tykk { (("┏", "┓"), ("┗", "┛"), "━", "┃") } else { (("╭", "╮"), ("╰", "╯"), "─", "│") };
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
    .map(|l| (l, if valgt && u.valgt == Valgt::Ramme { STERK } else { VANLIG }))
    .collect();
    match (i.starter, &i.branch) {
        (true, _) => inni.push(("starter …".to_string(), SVAK)),
        (false, Some(b)) => inni.push((kutt(b, indre), SVAK)),
        (false, None) => {}
    }
    let mut ut = vec![kantlinje(b, topp, strek, venstre, høyre, ramme)];
    for (tekst, stil) in inni {
        ut.push(vec![bit(0, side, ramme), bit(INNRYKK, tekst, stil), bit(b.saturating_sub(1), side, ramme)]);
    }
    ut.push(kantlinje(b, bunn, strek, Vec::new(), Vec::new(), ramme));
    ut
}

/// Øverste eller nederste kant i en boks `b` bred, med tekst til venstre og
/// høyre i streken.
fn kantlinje(b: u16, (v, h): (&str, &str), strek: &str, venstre: Kanttekst, høyre: Kanttekst, ramme: Style) -> Linje {
    let mut l = vec![bit(0, format!("{v}{strek}"), ramme)];
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
        l.push(bit(x, strek.repeat((slutt - x) as usize), ramme));
        x = slutt;
    }
    if !høyre.is_empty() {
        med_luft(&mut l, &mut x, høyre, None);
    }
    l.push(bit(x, format!("{strek}{h}"), ramme));
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
