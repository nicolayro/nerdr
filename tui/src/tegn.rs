//! Lista over paner til venstre og den valgte panen til høyre.

use nerdr_kjerne::paner::Pane;
use nerdr_kjerne::sgr::{Farge, Stil};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui::style::{Color, Modifier, Style};

use crate::grupper::{Merke, Rad};
use crate::{App, Fokus};

/// Bredden på lista, med skillelinja.
pub const LISTE: u16 = 36;
/// Teksten i et kort starter etter peker, glyf og et mellomrom.
const TEKST_X: u16 = 3;
const TEKST: usize = (LISTE - 1 - TEKST_X - 1) as usize;
const OPPGAVE_LINJER: usize = 2;

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

    if let (Fokus::Pane, Some((x, y))) = (app.fokus, app.markør) {
        let v = app.visning;
        if x < v.width && y < v.height {
            f.set_cursor_position((v.x + x, v.y + y));
        }
    }
}

fn tegn_liste(buf: &mut Buffer, område: Rect, app: &App) {
    let bredde = LISTE - 1;
    let hint_y = område.height.saturating_sub(1);

    let mut y = 0;
    for rad in app.rader() {
        if y >= hint_y {
            break;
        }
        match rad {
            Rad::Tom => y += 1,
            Rad::Overskrift(gruppe) => {
                let x = buf.set_stringn(1, y, gruppe.tittel(), bredde as usize, Style::new().fg(GRÅ)).0 + 1;
                let strek = "─".repeat(bredde.saturating_sub(x + 1) as usize);
                buf.set_stringn(x, y, strek, bredde as usize, Style::new().fg(DIM));
                y += 1;
            }
            Rad::Pane(pane, merke) => {
                let kort = kort(app, pane);
                let høyde = kort.høyde().min(hint_y - y);
                let valgt = app.valgt.as_ref().is_some_and(|v| v.id == pane.id);
                tegn_kort(buf, Rect::new(0, y, bredde, høyde), &kort, merke, valgt, app.fokus);
                y += høyde;
            }
        }
    }

    let hint = match app.fokus {
        Fokus::Liste => " ↑↓ velg · ⏎ gå inn · q avslutt",
        Fokus::Pane => " C-] tilbake til lista",
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
    let mappe = app
        .sesjoner
        .get(&pane.id)
        .and_then(|s| s.mappe.as_deref()?.file_name()?.to_str())
        .map_or_else(|| format!("{}:{}", pane.sesjon, pane.vindu), str::to_string);
    Kort {
        mappe,
        branch: app.brancher.get(&pane.id).cloned(),
        oppgave: pane.oppgave().map(|o| bryt(o, TEKST, OPPGAVE_LINJER)).unwrap_or_default(),
    }
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

/// Panen som står på rad `y` i lista.
pub fn pane_på(app: &App, y: u16) -> Option<Pane> {
    let mut topp = 0;
    for rad in app.rader() {
        let høyde = match &rad {
            Rad::Pane(pane, _) => kort(app, pane).høyde(),
            _ => 1,
        };
        if y < topp + høyde {
            return if let Rad::Pane(pane, _) = rad { Some(pane.clone()) } else { None };
        }
        topp += høyde;
    }
    None
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
        buf.set_stringn(v.x + 2, v.y + 1, "Ingen Claude-sesjoner i tmux.", v.width as usize, Style::new().fg(GRÅ));
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
