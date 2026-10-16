//! Lista over paner til venstre og den valgte panen til høyre.

use nerdr_kjerne::paner::Pane;
use nerdr_kjerne::sgr::{Farge, Linje, Stil};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui::style::{Color, Modifier, Style};

use crate::grupper::{Gruppe, Merke, Rad};
use crate::ny::Velger;
use crate::{App, Fokus, hjelp};

/// Bredden på lista, med skillelinja.
pub const LISTE: u16 = 36;
/// Teksten i et kort starter etter peker, glyf og to mellomrom. Glyfene har
/// tvetydig bredde i Unicode, og mange fonter tegner dem over to kolonner.
const TEKST_X: u16 = 4;
const TEKST: usize = (LISTE - 1 - TEKST_X - 1) as usize;
const OPPGAVE_LINJER: usize = 2;
const VELGER_LINJER: u16 = 12;

const ORANSJE: Color = Color::Rgb(217, 119, 87);
const GRØNN: Color = Color::Indexed(108);
const HVIT: Color = Color::Indexed(255);
const LYS: Color = Color::Indexed(252);
const GRÅ: Color = Color::Indexed(245);
const DIM: Color = Color::Indexed(240);
const MARKERT: Color = Color::Indexed(236);

pub fn visning(størrelse: Size) -> Rect {
    Rect::new(LISTE, 0, størrelse.width.saturating_sub(LISTE), størrelse.height)
}

pub fn alt(f: &mut Frame, app: &App) {
    let område = f.area();
    let buf = f.buffer_mut();
    tegn_liste(buf, område, app);
    tegn_pane(buf, app);

    if app.hjelp {
        tegn_hjelp(buf, område);
    } else if let Some(velger) = &app.velger {
        let markør = tegn_velger(buf, område, velger);
        f.set_cursor_position(markør);
    } else if let (Fokus::Pane, Some((x, y)), 0, Some(aktiv)) = (app.fokus, app.markør, app.rull, app.aktiv()) {
        let (x, y) = (aktiv.venstre + x, aktiv.topp + y);
        let v = app.visning;
        if x < v.width && y < v.height {
            f.set_cursor_position((v.x + x, v.y + y));
        }
    }
}

fn tegn_liste(buf: &mut Buffer, område: Rect, app: &App) {
    let bredde = LISTE - 1;
    let hint_y = område.height.saturating_sub(1);

    for (y, høyde, rad) in plasser(app, hint_y) {
        if y >= hint_y {
            break;
        }
        match rad {
            Rad::Tom => {}
            Rad::Strek => {
                let strek = "┈".repeat(bredde.saturating_sub(TEKST_X + 1) as usize);
                buf.set_stringn(TEKST_X, y, strek, TEKST, Style::new().fg(DIM));
            }
            Rad::Overskrift(gruppe) => {
                let x = buf.set_stringn(1, y, gruppe.tittel(), bredde as usize, Style::new().fg(GRÅ)).0 + 1;
                let strek = "─".repeat(bredde.saturating_sub(x + 1) as usize);
                buf.set_stringn(x, y, strek, bredde as usize, Style::new().fg(DIM));
            }
            Rad::Pane(pane, merke) => {
                let kort = kort(app, pane);
                let område = Rect::new(0, y, bredde, høyde.min(hint_y - y));
                let valgt = app.valgt.as_ref().is_some_and(|v| v.id == pane.id);
                tegn_kort(buf, område, &kort, merke, valgt, app.fokus);
            }
            Rad::Terminal(pane) => {
                let valgt = app.valgt.as_ref().is_some_and(|v| v.id == pane.id);
                tegn_terminal(buf, Rect::new(0, y, bredde, 1), pane, valgt, app.fokus);
            }
        }
    }

    let hint = match app.fokus {
        Fokus::Liste => " ↑↓ velg · ⏎ inn · n ny · ? hjelp",
        Fokus::Pane => " C-q tilbake til lista",
    };
    buf.set_stringn(0, hint_y, hint, bredde as usize, Style::new().fg(GRÅ));

    for y in 0..område.height {
        buf.set_stringn(bredde, y, "│", 1, Style::new().fg(DIM));
    }
}

/// En sesjon i lista: mappe › branch, og oppgaven under.
struct Kort {
    mappe: String,
    branch: Option<String>,
    oppgave: Vec<String>,
}

impl Kort {
    fn høyde(&self) -> u16 {
        1 + self.oppgave.len() as u16
    }
}

fn kort(app: &App, pane: &Pane) -> Kort {
    let ny = app.nye.get(&pane.id);
    let mappe = app
        .sesjoner
        .get(&pane.id)
        .and_then(|s| s.mappe.as_deref())
        .or(ny.map(|m| m.as_path()))
        .and_then(|m| m.file_name()?.to_str())
        .map_or_else(|| format!("{}:{}", pane.sesjon, pane.vindu), str::to_string);
    let oppgave = match ny {
        Some(_) => vec!["starter …".to_string()],
        None => pane.oppgave().map(|o| bryt(o, TEKST, OPPGAVE_LINJER)).unwrap_or_default(),
    };
    Kort { mappe, branch: app.brancher.get(&pane.id).cloned(), oppgave }
}

fn tegn_kort(buf: &mut Buffer, område: Rect, kort: &Kort, merke: Merke, valgt: bool, fokus: Fokus) {
    tegn_peker(buf, område, valgt, fokus);
    let farge = if valgt { HVIT } else { LYS };
    let y = område.y;
    buf.set_stringn(1, y, merke.glyf(), 1, Style::new().fg(glyffarge(merke)));

    let mappe = kutt(&kort.mappe, TEKST);
    let x = buf.set_stringn(TEKST_X, y, &mappe, TEKST, Style::new().fg(farge).add_modifier(Modifier::BOLD)).0;
    if let Some(branch) = &kort.branch {
        let rest = TEKST.saturating_sub(mappe.chars().count());
        buf.set_stringn(x, y, kutt(&format!(" › {branch}"), rest), rest, Style::new().fg(GRÅ));
    }

    let oppgavefarge = if valgt { LYS } else { GRÅ };
    for (linje, y) in kort.oppgave.iter().zip(y + 1..område.bottom()) {
        buf.set_stringn(TEKST_X, y, linje, TEKST, Style::new().fg(oppgavefarge));
    }
}

/// Programmet som kjører i terminalen, under kortet til Claude-en.
fn tegn_terminal(buf: &mut Buffer, område: Rect, pane: &Pane, valgt: bool, fokus: Fokus) {
    tegn_peker(buf, område, valgt, fokus);
    let x = buf.set_stringn(TEKST_X, område.y, "└ ", TEKST, Style::new().fg(DIM)).0;
    let rest = TEKST.saturating_sub(2);
    let farge = if valgt { LYS } else { GRÅ };
    buf.set_stringn(x, område.y, kutt(&pane.kommando, rest), rest, Style::new().fg(farge));
}

fn tegn_peker(buf: &mut Buffer, område: Rect, valgt: bool, fokus: Fokus) {
    if valgt && fokus == Fokus::Liste {
        buf.set_style(område, Style::new().bg(MARKERT));
    }
    let peker = if valgt { "▎" } else { " " };
    for y in område.top()..område.bottom() {
        buf.set_stringn(0, y, peker, 1, Style::new().fg(HVIT));
    }
}

fn glyffarge(merke: Merke) -> Color {
    match merke {
        Merke::Spør | Merke::Ferdig => ORANSJE,
        Merke::Jobber | Merke::Starter => GRØNN,
        Merke::Sett => LYS,
    }
}

/// Radene i lista med y og høyde. Får alt plass, legges SETT nederst, med en
/// tom rad over hintet, så det du har lagt bort står for seg selv.
fn plasser<'a>(app: &'a App, plass: u16) -> Vec<(u16, u16, Rad<'a>)> {
    let rader = app.rader();
    let høyder: Vec<u16> = rader
        .iter()
        .map(|r| if let Rad::Pane(pane, _) = r { kort(app, pane).høyde() } else { 1 })
        .collect();
    let sett = rader.iter().position(|r| matches!(r, Rad::Overskrift(Gruppe::Sett))).unwrap_or(rader.len());
    let over: u16 = høyder[..sett].iter().sum();
    let under: u16 = høyder[sett..].iter().sum();
    let sett_y = if over + under < plass { plass - under - 1 } else { over };

    let mut y = 0;
    let mut plassert = Vec::with_capacity(rader.len());
    for (i, (rad, høyde)) in rader.into_iter().zip(høyder).enumerate() {
        if i == sett {
            y = sett_y;
        }
        plassert.push((y, høyde, rad));
        y += høyde;
    }
    plassert
}

/// Panen som står på rad `y` i lista.
pub fn pane_på(app: &App, y: u16) -> Option<Pane> {
    let plass = app.visning.height.saturating_sub(1);
    plasser(app, plass).into_iter().find_map(|(topp, høyde, rad)| match rad {
        Rad::Pane(pane, _) | Rad::Terminal(pane) if (topp..topp + høyde).contains(&y) => Some(pane.clone()),
        _ => None,
    })
}

/// En boks med ramme midt på skjermen, med tittel øverst og hint nederst,
/// eller `None` når skjermen er for liten.
fn boks(buf: &mut Buffer, område: Rect, bredde: u16, høyde: u16, tittel: &str, hint: &str) -> Option<Rect> {
    let bredde = område.width.saturating_sub(4).min(bredde);
    let høyde = område.height.saturating_sub(2).min(høyde);
    if bredde < 12 || høyde < 5 {
        return None;
    }
    let boks = Rect::new((område.width - bredde) / 2, (område.height - høyde) / 3, bredde, høyde);
    let ramme = Style::new().fg(ORANSJE);
    buf.set_style(boks, Style::reset());
    for y in boks.top()..boks.bottom() {
        let (v, fyll, h) = match y {
            y if y == boks.top() => ("╭", "─", "╮"),
            y if y == boks.bottom() - 1 => ("╰", "─", "╯"),
            _ => ("│", " ", "│"),
        };
        buf.set_string(boks.x, y, format!("{v}{}{h}", fyll.repeat(bredde as usize - 2)), ramme);
    }
    let tekst = (bredde - 4) as usize;
    buf.set_stringn(boks.x + 2, boks.y, format!(" {tittel} "), tekst, Style::new().fg(HVIT).add_modifier(Modifier::BOLD));
    buf.set_stringn(boks.x + 2, boks.bottom() - 1, format!(" {hint} "), tekst, Style::new().fg(GRÅ));
    Some(boks)
}

fn tegn_hjelp(buf: &mut Buffer, område: Rect) {
    const BREDDE: u16 = 58;
    let indre = (BREDDE - 4) as usize;
    let om = bryt(hjelp::OM, indre, 3);
    let høyde = om.len() + 1 + hjelp::TASTER.len() + 1 + hjelp::MERKER.len() + 1 + 2 + 2;
    let Some(boks) = boks(buf, område, BREDDE, høyde as u16, "nerdr", "trykk en tast for å lukke") else { return };
    let x = boks.x + 2;
    let indre = (boks.width - 4) as usize;
    let mut y = boks.y + 1..boks.bottom() - 1;
    let mut linje = |tekst: &str, stil: Style| {
        if let Some(y) = y.next() {
            buf.set_stringn(x, y, tekst, indre, stil);
        }
    };
    for l in om {
        linje(&l, Style::new().fg(LYS));
    }
    linje("", Style::new());
    for (tast, hva) in hjelp::TASTER {
        linje(&format!("{tast:<9}{hva}"), Style::new().fg(LYS));
    }
    linje("", Style::new());
    for (merke, hva) in hjelp::MERKER {
        linje(&format!("{}  {hva}", merke.glyf()), Style::new().fg(LYS));
    }
    linje("", Style::new());
    linje(hjelp::BARE_TMUX, Style::new().fg(GRÅ));
    linje(&hjelp::mapper(), Style::new().fg(GRÅ));
}

/// Mappevelgeren midt på skjermen. Gir der markøren skal stå i søkefeltet.
fn tegn_velger(buf: &mut Buffer, område: Rect, velger: &Velger) -> (u16, u16) {
    let (tittel, hint) = match velger.fortsett {
        false => ("ny claude", "⏎ start · C-r tidligere samtaler · esc avbryt"),
        true => ("tidligere samtaler", "⏎ fortsett · C-r ny · esc avbryt"),
    };
    let Some(boks) = boks(buf, område, 64, VELGER_LINJER + 4, tittel, hint) else {
        return (0, 0);
    };
    let bredde = boks.width;
    let høyde = boks.height;
    let indre = (bredde - 4) as usize;

    let treff = velger.treff();
    let x = boks.x + 2;
    let y = boks.y + 1;
    let prompt = if velger.fortsett { "samtale › " } else { "mappe › " };
    let søk = kutt(&velger.søk, indre.saturating_sub(prompt.chars().count() + 8));
    let etter = buf.set_stringn(x, y, prompt, indre, Style::new().fg(ORANSJE)).0;
    let markør = buf.set_stringn(etter, y, &søk, indre, Style::new().fg(HVIT)).0;
    let antall = format!("{}/{}", treff.len(), velger.antall());
    buf.set_string(boks.right() - 2 - antall.chars().count() as u16, y, antall, Style::new().fg(DIM));

    let linjer = (høyde - 3) as usize;
    let fra = velger.valgt.saturating_sub(linjer - 1);
    for (i, t) in treff.iter().enumerate().skip(fra).take(linjer) {
        let y = boks.y + 2 + (i - fra) as u16;
        let valgt = i == velger.valgt;
        if valgt {
            buf.set_style(Rect::new(boks.x + 1, y, bredde - 2, 1), Style::new().bg(MARKERT));
            buf.set_string(boks.x + 1, y, "▎", Style::new().fg(HVIT));
        }
        let farge = if valgt { HVIT } else { LYS };
        let k = t.kandidat;
        let mut plass = indre;
        if let Some(alder) = &k.alder {
            let bredde = alder.chars().count();
            buf.set_string(boks.right() - 2 - bredde as u16, y, alder, Style::new().fg(DIM));
            plass = plass.saturating_sub(bredde + 2);
        }
        // Mappenavnet foran tittelen til en samtale er grått.
        let mappe = k.samtale.as_ref().and_then(|_| k.vis.find("  ")).map_or(0, |i| k.vis[..i].chars().count());
        for (n, c) in kutt(&k.vis, plass).chars().enumerate() {
            let farge = match () {
                _ if t.tegn.contains(&n) => ORANSJE,
                _ if n < mappe => GRÅ,
                _ => farge,
            };
            buf.set_string(x + n as u16, y, c.encode_utf8(&mut [0; 4]), Style::new().fg(farge));
        }
    }
    (markør, y)
}

fn kutt(tekst: &str, bredde: usize) -> String {
    if tekst.chars().count() <= bredde {
        return tekst.to_string();
    }
    let mut kuttet: String = tekst.chars().take(bredde.saturating_sub(1)).collect();
    kuttet.push('…');
    kuttet
}

/// Bryter på mellomrom over høyst `maks` linjer, og ender med … når teksten
/// ikke får plass.
fn bryt(tekst: &str, bredde: usize, maks: usize) -> Vec<String> {
    let mut linjer: Vec<String> = Vec::new();
    for ord in tekst.split_whitespace() {
        match linjer.last_mut() {
            Some(l) if l.chars().count() + 1 + ord.chars().count() <= bredde => {
                l.push(' ');
                l.push_str(ord);
            }
            _ => linjer.push(ord.to_string()),
        }
    }
    let for_lang = linjer.len() > maks;
    linjer.truncate(maks);
    let mut linjer: Vec<String> = linjer.iter().map(|l| kutt(l, bredde)).collect();
    if for_lang && let Some(siste) = linjer.last_mut() {
        *siste = siste.chars().take(bredde.saturating_sub(1)).chain(['…']).collect();
    }
    linjer
}

fn tegn_pane(buf: &mut Buffer, app: &App) {
    let v = app.visning;
    if v.width == 0 {
        return;
    }
    if app.valgt.is_none() {
        buf.set_stringn(v.x + 2, v.y + 1, "Ingen Claude-sesjoner i tmux. n starter en ny.", v.width as usize, Style::new().fg(GRÅ));
        return;
    }
    for pane in &app.vindu {
        let område = Rect::new(v.x + pane.venstre, v.y + pane.topp, pane.bredde, pane.høyde).intersection(v);
        if let Some(bilde) = app.bilder.get(&pane.id) {
            tegn_bilde(buf, område, bilde);
        }
    }
    if app.vindu.len() > 1 {
        tegn_rammer(buf, app);
    }
    if let (true, Some(aktiv)) = (app.rull > 0, app.aktiv()) {
        let hvor = format!(" {}/{} ", app.rull, app.historie);
        let bredde = hvor.chars().count() as u16;
        let (høyre, y) = (v.x + aktiv.venstre + aktiv.bredde, v.y + aktiv.topp);
        if bredde < aktiv.bredde && høyre <= v.right() && y < v.bottom() {
            buf.set_string(høyre - bredde, y, hvor, Style::new().fg(ORANSJE).add_modifier(Modifier::REVERSED));
        }
    }
}

fn tegn_bilde(buf: &mut Buffer, område: Rect, bilde: &[Linje]) {
    for (y, linje) in (område.top()..område.bottom()).zip(bilde) {
        let mut x = område.x;
        for bit in linje {
            if x >= område.right() {
                break;
            }
            x = buf.set_stringn(x, y, &bit.tekst, (område.right() - x) as usize, stil(bit.stil)).0;
        }
    }
}

/// Rutene i vinduet som ingen pane dekker, er rammene tmux legger mellom
/// dem. Tegnet i hver rute avhenger av hvilke naboer som også er ramme, så
/// kryss og hjørner blir riktige. Rammen rundt den aktive panen er oransje
/// når du er inne i vinduet.
fn tegn_rammer(buf: &mut Buffer, app: &App) {
    let v = app.visning;
    let første = &app.vindu[0];
    let (bredde, høyde) = (første.vindu_bredde.min(v.width), første.vindu_høyde.min(v.height));
    let dekket = |x: u16, y: u16| {
        app.vindu.iter().any(|p| (p.venstre..p.venstre + p.bredde).contains(&x) && (p.topp..p.topp + p.høyde).contains(&y))
    };
    let ramme = |x: i32, y: i32| {
        x >= 0 && y >= 0 && x < bredde as i32 && y < høyde as i32 && !dekket(x as u16, y as u16)
    };
    let aktiv = app.aktiv().filter(|_| app.fokus == Fokus::Pane);
    let ved_aktiv = |x: u16, y: u16| {
        aktiv.is_some_and(|p| x + 1 >= p.venstre && x <= p.venstre + p.bredde && y + 1 >= p.topp && y <= p.topp + p.høyde)
    };
    for y in 0..høyde {
        for x in 0..bredde {
            if dekket(x, y) {
                continue;
            }
            let (xi, yi) = (x as i32, y as i32);
            let tegn = rammetegn(ramme(xi, yi - 1), ramme(xi, yi + 1), ramme(xi - 1, yi), ramme(xi + 1, yi));
            let farge = if ved_aktiv(x, y) { ORANSJE } else { DIM };
            buf.set_string(v.x + x, v.y + y, tegn, Style::new().fg(farge));
        }
    }
}

fn rammetegn(opp: bool, ned: bool, venstre: bool, høyre: bool) -> &'static str {
    match (opp, ned, venstre, høyre) {
        (true, true, true, true) => "┼",
        (true, true, false, true) => "├",
        (true, true, true, false) => "┤",
        (false, true, true, true) => "┬",
        (true, false, true, true) => "┴",
        (false, true, false, true) => "┌",
        (false, true, true, false) => "┐",
        (true, false, false, true) => "└",
        (true, false, true, false) => "┘",
        (_, _, false, false) => "│",
        _ => "─",
    }
}

fn stil(s: Stil) -> Style {
    let mut stil = Style::new().fg(farge(s.fg)).bg(farge(s.bg));
    for (på, m) in [
        (s.fet, Modifier::BOLD),
        (s.svak, Modifier::DIM),
        (s.kursiv, Modifier::ITALIC),
        (s.understrek, Modifier::UNDERLINED),
        (s.blink, Modifier::SLOW_BLINK),
        (s.invers, Modifier::REVERSED),
        (s.skjult, Modifier::HIDDEN),
        (s.overstrek, Modifier::CROSSED_OUT),
    ] {
        if på {
            stil = stil.add_modifier(m);
        }
    }
    stil
}

fn farge(f: Farge) -> Color {
    match f {
        Farge::Standard => Color::Reset,
        Farge::Indeks(i) => Color::Indexed(i),
        Farge::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn rammetegn_etter_naboene() {
        assert_eq!(rammetegn(true, true, false, false), "│");
        assert_eq!(rammetegn(false, false, true, true), "─");
        assert_eq!(rammetegn(true, true, false, true), "├");
        assert_eq!(rammetegn(false, false, false, false), "│");
    }

    #[test]
    fn bryter_på_ord_og_kutter_det_som_ikke_får_plass() {
        assert_eq!(bryt("Fiks testene", 31, 2), ["Fiks testene"]);
        assert_eq!(
            bryt("Forbindelser i Newcustomere2elocal for case", 31, 2),
            ["Forbindelser i", "Newcustomere2elocal for case"]
        );
        assert_eq!(bryt("en to tre fire", 7, 2), ["en to", "tre…"]);
        assert_eq!(bryt("Langtordsomikkefårplass", 10, 2), ["Langtords…"]);
    }
}
