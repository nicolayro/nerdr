//! Lista over paner til venstre og den valgte panen til høyre.

use nerdr_kjerne::paner::Pane;
use nerdr_kjerne::sgr::{Farge, Stil};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui::style::{Color, Modifier, Style};

use crate::grupper::{Gruppe, Merke, Rad};
use crate::ny::Velger;
use crate::{App, Fokus};

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

    if let Some(velger) = &app.velger {
        let markør = tegn_velger(buf, område, velger);
        f.set_cursor_position(markør);
    } else if let (Fokus::Pane, Some((x, y))) = (app.fokus, app.markør) {
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
        }
    }

    let hint = match app.fokus {
        Fokus::Liste => " ↑↓ velg · ⏎ inn · n ny · q avslutt",
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
    if valgt && fokus == Fokus::Liste {
        buf.set_style(område, Style::new().bg(MARKERT));
    }
    let (peker, farge) = if valgt { ("▎", HVIT) } else { (" ", LYS) };
    for y in område.top()..område.bottom() {
        buf.set_stringn(0, y, peker, 1, Style::new().fg(HVIT));
    }
    let (glyf, glyffarge) = match merke {
        Merke::Spør => ("●", ORANSJE),
        Merke::Ferdig => ("◆", ORANSJE),
        Merke::Jobber => ("◐", GRØNN),
        Merke::Sett => ("○", LYS),
        Merke::Starter => ("◌", GRØNN),
    };
    let y = område.y;
    buf.set_stringn(1, y, glyf, 1, Style::new().fg(glyffarge));

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
        Rad::Pane(pane, _) if (topp..topp + høyde).contains(&y) => Some(pane.clone()),
        _ => None,
    })
}

/// Mappevelgeren midt på skjermen. Gir der markøren skal stå i søkefeltet.
fn tegn_velger(buf: &mut Buffer, område: Rect, velger: &Velger) -> (u16, u16) {
    let bredde = område.width.saturating_sub(4).min(64);
    let høyde = område.height.saturating_sub(4).min(VELGER_LINJER + 4);
    let boks = Rect::new((område.width - bredde) / 2, (område.height - høyde) / 3, bredde, høyde);
    if bredde < 12 || høyde < 5 {
        return (boks.x, boks.y);
    }
    let indre = (bredde - 4) as usize;
    let ramme = Style::new().fg(ORANSJE);
    buf.set_style(boks, Style::reset());
    for y in boks.top()..boks.bottom() {
        let (v, h) = match y {
            y if y == boks.top() => ("╭", "╮"),
            y if y == boks.bottom() - 1 => ("╰", "╯"),
            _ => ("│", "│"),
        };
        let fyll = if y == boks.top() || y == boks.bottom() - 1 { "─" } else { " " };
        buf.set_string(boks.x, y, format!("{v}{}{h}", fyll.repeat(bredde as usize - 2)), ramme);
    }
    buf.set_string(boks.x + 2, boks.y, " ny claude ", Style::new().fg(HVIT).add_modifier(Modifier::BOLD));
    buf.set_string(boks.x + 2, boks.bottom() - 1, " ⏎ start · esc avbryt ", Style::new().fg(GRÅ));

    let treff = velger.treff();
    let x = boks.x + 2;
    let y = boks.y + 1;
    let prompt = "mappe › ";
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
        for (n, c) in kutt(&t.kandidat.vis, indre).chars().enumerate() {
            let stil = if t.tegn.contains(&n) { Style::new().fg(ORANSJE) } else { Style::new().fg(farge) };
            buf.set_string(x + n as u16, y, c.encode_utf8(&mut [0; 4]), stil);
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
    for (y, linje) in app.bilde.iter().enumerate().take(v.height as usize) {
        let y = v.y + y as u16;
        let mut x = v.x;
        for bit in linje {
            if x >= v.right() {
                break;
            }
            x = buf.set_stringn(x, y, &bit.tekst, (v.right() - x) as usize, stil(bit.stil)).0;
        }
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
