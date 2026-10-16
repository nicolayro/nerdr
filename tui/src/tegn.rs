//! Lista over paner til venstre og den valgte panen til høyre.

use nerdr_kjerne::sgr::{Farge, Stil};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui::style::{Color, Modifier, Style};

use crate::grupper::{Merke, Rad};
use crate::{App, Fokus};

/// Bredden på lista, med skillelinja.
pub const LISTE: u16 = 36;

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

    for (y, rad) in app.rader().into_iter().enumerate().take(hint_y as usize) {
        let y = y as u16;
        let (pane, merke) = match rad {
            Rad::Tom => continue,
            Rad::Overskrift(gruppe) => {
                let x = buf.set_stringn(1, y, gruppe.tittel(), bredde as usize, Style::new().fg(GRÅ)).0 + 1;
                let strek = "─".repeat(bredde.saturating_sub(x + 1) as usize);
                buf.set_stringn(x, y, strek, bredde as usize, Style::new().fg(DIM));
                continue;
            }
            Rad::Pane(pane, merke) => (pane, merke),
        };
        let valgt = app.valgt.as_ref().is_some_and(|v| v.id == pane.id);
        if valgt && app.fokus == Fokus::Liste {
            buf.set_style(Rect::new(0, y, bredde, 1), Style::new().bg(MARKERT));
        }
        let (peker, farge) = if valgt { ("▎", HVIT) } else { (" ", LYS) };
        let (glyf, glyffarge) = match merke {
            Merke::Spør => ("●", ORANSJE),
            Merke::Ferdig => ("◆", ORANSJE),
            Merke::Jobber => ("◐", GRØNN),
            Merke::Sett => ("○", LYS),
        };
        let navn = format!("{}:{} ", pane.sesjon, pane.vindu);
        let x = buf.set_stringn(0, y, peker, 1, Style::new().fg(HVIT)).0;
        let x = buf.set_stringn(x, y, format!("{glyf} "), 2, Style::new().fg(glyffarge)).0;
        let x = buf.set_stringn(x, y, &navn, (bredde - x) as usize, Style::new().fg(farge)).0;
        buf.set_stringn(x, y, &pane.tittel, bredde.saturating_sub(x + 1) as usize, Style::new().fg(DIM));
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
