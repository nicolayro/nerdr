//! Sidebaren og panen, tegnet med ratatui.

use nerdr_kjerne::oversikt::{self, Rad};
use nerdr_kjerne::sgr::{Farge, Stil};
use nerdr_kjerne::tilstand::Tilstand;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui::style::{Color, Modifier, Style};

use crate::{App, Fokus};

/// Bredden på sidebaren, med skillelinja.
pub const SIDEBAR: u16 = 36;

const ORANSJE: Color = Color::Rgb(217, 119, 87);
const GRØNN: Color = Color::Indexed(108);
const BLÅ: Color = Color::Indexed(110);
const LYS: Color = Color::Indexed(252);
const HVIT: Color = Color::Indexed(255);
const GRÅ: Color = Color::Indexed(245);
const DIM: Color = Color::Indexed(240);
const MARKERT: Color = Color::Indexed(236);

pub struct Sidelinje {
    venstre: Vec<(String, Style)>,
    høyre: Option<(String, Style)>,
    markert: bool,
    /// Panen et klikk på linja går til.
    pub mål: Option<String>,
}

pub fn visning(størrelse: Size) -> Rect {
    Rect::new(SIDEBAR, 0, størrelse.width.saturating_sub(SIDEBAR), størrelse.height)
}

pub fn sidebar(app: &App) -> Vec<Sidelinje> {
    let mut ut = Vec::new();
    let tom = |mål| Sidelinje { venstre: Vec::new(), høyre: None, markert: false, mål };

    let venter = oversikt::telling(&app.grupper, Tilstand::Venter);
    // Bare det du kan overse, får en overskrift. Hvor mange som jobber eller er
    // ledige, leser du av glyfene i lista.
    if venter > 0 {
        ut.push(Sidelinje {
            venstre: vec![(format!(" ● {venter} venter på deg"), Style::new().fg(ORANSJE).bold())],
            ..tom(None)
        });
    }

    for gruppe in &app.grupper {
        ut.push(tom(None));
        // Klikk på repo-navnet går til raden i gruppa som trenger deg mest.
        let viktigst = gruppe.rader.iter().max_by_key(|r| r.tilstand.rang());
        ut.push(Sidelinje {
            venstre: vec![(format!(" {}", gruppe.repo), Style::new().fg(BLÅ))],
            ..tom(viktigst.map(|r| r.pane.id.clone()))
        });
        for rad in &gruppe.rader {
            ut.extend(radlinjer(app, rad));
        }
    }
    ut
}

fn radlinjer(app: &App, rad: &Rad) -> Vec<Sidelinje> {
    let id = &rad.pane.id;
    let vist = app.vist.as_ref() == Some(id);
    let (glyf, farge) = match rad.tilstand {
        Tilstand::Venter => ("●", ORANSJE),
        Tilstand::Jobber => ("◐", GRØNN),
        Tilstand::Klar => ("○", LYS),
        Tilstand::Shell => ("·", DIM),
    };
    let hoved = rad.hovedtekst();
    let tekstfarge = if !rad.spørsmål.is_empty() {
        ORANSJE
    } else if vist {
        HVIT
    } else {
        LYS
    };
    let markert = app.fokus == Fokus::Liste && app.valgt.as_ref() == Some(id);

    let mut linjer = vec![Sidelinje {
        venstre: vec![
            (if vist { "▎" } else { " " }.into(), Style::new().fg(HVIT)),
            (glyf.into(), Style::new().fg(farge)),
            (format!(" {hoved}"), Style::new().fg(tekstfarge)),
        ],
        høyre: Some((
            rad.status.clone(),
            Style::new().fg(if rad.tilstand == Tilstand::Venter { ORANSJE } else { DIM }),
        )),
        markert,
        mål: Some(id.clone()),
    }];
    // Worktreet sier hvilken utsjekk raden gjelder når flere sesjoner deler
    // repo. Sløyfes når hovedteksten alt sier det samme.
    if !rad.worktree.is_empty() && rad.worktree != hoved {
        linjer.push(Sidelinje {
            venstre: vec![(format!("   {}", rad.worktree), Style::new().fg(DIM))],
            høyre: None,
            markert,
            mål: Some(id.clone()),
        });
    }
    linjer
}

pub fn alt(f: &mut Frame, app: &App, sidelinjer: &[Sidelinje]) {
    let område = f.area();
    let buf = f.buffer_mut();
    tegn_sidebar(buf, område, app, sidelinjer);
    tegn_pane(buf, app);

    if let (Fokus::Pane, Some((x, y))) = (app.fokus, app.markør) {
        let v = app.visning;
        if x < v.width && y < v.height {
            f.set_cursor_position((v.x + x, v.y + y));
        }
    }
}

fn tegn_sidebar(buf: &mut Buffer, område: Rect, app: &App, linjer: &[Sidelinje]) {
    let innhold = SIDEBAR - 1;
    let hint_y = område.height.saturating_sub(1);

    for (y, linje) in linjer.iter().enumerate().take(hint_y as usize) {
        let y = y as u16;
        if linje.markert {
            buf.set_style(Rect::new(0, y, innhold, 1), Style::new().bg(MARKERT));
        }
        let høyre_bredde = linje.høyre.as_ref().map_or(0, |(t, _)| t.chars().count() as u16 + 1);
        let grense = innhold.saturating_sub(høyre_bredde + 1);
        let mut x = 0;
        for (tekst, stil) in &linje.venstre {
            x = buf.set_stringn(x, y, tekst, grense.saturating_sub(x) as usize, *stil).0;
        }
        if let Some((tekst, stil)) = &linje.høyre {
            buf.set_stringn(innhold - høyre_bredde, y, tekst, høyre_bredde as usize, *stil);
        }
    }

    let hint = match app.fokus {
        Fokus::Liste => " ↑↓ velg · ⏎ gå inn · q avslutt",
        Fokus::Pane => " C-] tilbake til lista",
    };
    buf.set_stringn(0, hint_y, hint, innhold as usize, Style::new().fg(GRÅ));

    for y in 0..område.height {
        buf.set_stringn(innhold, y, "│", 1, Style::new().fg(DIM));
    }
}

fn tegn_pane(buf: &mut Buffer, app: &App) {
    let v = app.visning;
    if v.width == 0 {
        return;
    }
    if app.vist.is_none() {
        let melding = if app.grupper.is_empty() { "Ingen tmux-sesjoner." } else { "Velg en sesjon." };
        buf.set_stringn(v.x + 2, v.y + 1, melding, v.width as usize, Style::new().fg(GRÅ));
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
