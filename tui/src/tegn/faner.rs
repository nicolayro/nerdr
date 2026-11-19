//! Fanene over visningen.

use nerdr_kjerne::merke::Merke;
use nerdr_kjerne::paner::Pane;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use std::ops::Range;
use crate::App;
use crate::bokser::Rammer;
use crate::grupper::glyf;
use super::*;

/// Fanene og streken under dem. Med bokser er streken toppkanten på boksene,
/// og fanene har en ramme over seg.
pub(super) const FANERADER: u16 = 2;

/// Raden fanene står på.
pub(super) fn fanerad(app: &App) -> u16 {
    (app.utseende.paner != Rammer::Linjer) as u16
}

/// Radene øverst i agentlista: versjonen og hjelp, og ny sesjon og
/// sorteringen, med streken under.
pub(super) const TOPPEN: u16 = 2;

pub(super) const NY_RAD: u16 = 1;

/// Kolonnene fanene har plass i, innenfor rammen med bokser.
pub(super) fn fanerom(app: &App) -> Range<u16> {
    let v = app.visning;
    match app.utseende.paner {
        Rammer::Bokser => v.x + 2..v.right().saturating_sub(1),
        Rammer::Linjer | Rammer::Vindu => v.x + 1..v.right(),
    }
}

/// Et vindu i sesjonen, som fane over panene.
pub(super) struct Fane {
    pub(super) fra: u16,
    pub(super) til: u16,
    pub(super) navn: String,
    pub(super) vindu: String,
    pub(super) vist: bool,
    pub(super) ikon: Ikon,
    pub(super) zoomet: bool,
}

/// Tegnet foran navnet på fanen, og kolonnene til navnet.
pub(super) struct Ikon {
    pub(super) tegn: String,
    pub(super) stil: Style,
    pub(super) bredde: u16,
}

/// Luft til høyre for navnet, med `×` i.
pub(super) const FANELUFT: u16 = 3;

/// Tegnet foran navnet: merket til Claude-en, eller den som haster mest når
/// det er flere i vinduet, og ellers nummeret på vinduet. Glyfene har
/// tvetydig bredde i Unicode, og mange fonter tegner dem bredere enn
/// sifrene, så de får et mellomrom ekstra.
pub(super) fn ikon(app: &App, p: &Pane) -> Ikon {
    let haster = app
        .alle
        .iter()
        .filter(|q| q.vindu_id == p.vindu_id)
        .filter_map(|q| app.merke(q))
        .min_by_key(|m| (m.gruppe(), *m != Merke::Spør));
    let (tegn, stil) = match haster {
        Some(m) => (glyf(m).to_string(), kort::glyfstil(m)),
        None => (p.vindu.clone(), SVAK),
    };
    let bredde = tegn.chars().count() as u16 + 1 + haster.is_some() as u16;
    Ikon { tegn, stil, bredde }
}

impl Fane {
    /// Kolonnen til `×`. Plassen står i fanen også når `×` ikke vises, så
    /// fanene ikke endrer bredde når musen kommer.
    pub(super) fn lukk(&self) -> u16 {
        self.til - 2
    }
}

/// Etter navnet på fanen, som i statuslinja til tmux.
pub(super) const ZOOM: &str = " Z";

/// Fanene som får plass. De andre tegnes ikke, så de kan heller ikke treffes.
pub(super) fn synlige_faner(app: &App) -> Vec<Fane> {
    let slutt = fanerom_til_fanene(app).end;
    faner(app).into_iter().take_while(|f| f.til < slutt).collect()
}

pub(super) fn faner(app: &App) -> Vec<Fane> {
    let mut x = fanerom(app).start;
    let mut ut = Vec::new();
    for p in app.faner() {
        let navn = kutt(&p.vindunavn, 20);
        let ikon = ikon(app, p);
        let zoom = if p.zoomet { ZOOM.len() as u16 } else { 0 };
        let til = x + 1 + ikon.bredde + navn.chars().count() as u16 + zoom + FANELUFT;
        let vist = app.vist.as_ref() == Some(&p.vindu_id);
        ut.push(Fane { fra: x, til, navn, vindu: p.vindu_id.clone(), vist, ikon, zoomet: p.zoomet });
        x = til;
    }
    ut
}

/// Vinduene i sesjonen som faner med en strek under, og med bokser i en ramme
/// over panene, der streken er toppkanten på boksene. Fanen som vises, er en
/// lapp med bakgrunn, og den og fanen under musen har `×`. Tegnes etter
/// panene, fordi streken går inn i toppkanten på boksene.
pub(super) fn tegn_faner(buf: &mut Buffer, app: &App, mus: Option<&Mål>) {
    let v = app.visning;
    let rad = fanerad(app);
    let boks = app.utseende.paner == Rammer::Bokser;
    let faner = synlige_faner(app);
    let y = v.y - 1 + boks as u16;
    // Rammen over fanene har samme farge som boksen under.
    let ramme = if boks && buf[(v.x, y)].symbol() == "╭" { buf[(v.x, y)].style() } else { SVAK };
    // Ved siden av rammen over fanene står skillelinja for seg.
    let fra = if boks { v.x } else { v.x.saturating_sub(1) };
    for x in fra..v.right() {
        let rute = &buf[(x, y)];
        let (symbol, stil) = (rute.symbol().to_string(), rute.style());
        let skillelinja = x + 1 == v.x;
        // Merkelappen på rammen, som « kopi », står.
        if !skillelinja && (!matches!(symbol.as_str(), " " | "─" | "╭" | "╮" | "┬") || stil.add_modifier.contains(Modifier::REVERSED)) {
            continue;
        }
        let fra_boks = matches!(symbol.as_str(), "╭" | "╮" | "┬");
        let kant = boks && (x == v.x || x + 1 == v.right());
        let (opp, ned) = if skillelinja { (true, true) } else { (kant, fra_boks) };
        let venstre = match () {
            _ if skillelinja => matches!(symbol.as_str(), "┤" | "┼"),
            _ => !(boks && x == v.x),
        };
        let høyre = x + 1 < v.right();
        let tegn = match (opp, ned, venstre, høyre) {
            (false, true, false, true) => "╭",
            (false, true, true, false) => "╮",
            (o, n, v, h) => rammetegn(o, n, v, h),
        };
        // Der en loddrett kant går gjennom streken, har den fargen til rammen,
        // ellers ser kanten brutt ut.
        let stil = match () {
            _ if skillelinja => stil,
            _ if fra_boks => stil,
            _ if kant => ramme,
            _ if boks => skillestil(),
            _ => SVAK,
        };
        buf[(x, y)].set_symbol(tegn).set_style(stil);
    }
    if boks && v.width >= 2 {
        buf.set_string(v.x, 0, format!("╭{}╮", "─".repeat(v.width as usize - 2)), ramme);
        buf.set_string(v.x, rad, "│", ramme);
        buf.set_string(v.right() - 1, rad, "│", ramme);
    }
    for f in &faner {
        let på_kryss = matches!(mus, Some(Mål::LukkFane(vindu)) if *vindu == f.vindu);
        let pekt = på_kryss || matches!(mus, Some(Mål::Fane(vindu)) if *vindu == f.vindu);
        let (tekst, lapp) = match () {
            _ if f.vist => (STERK, markert()),
            _ if pekt => (VANLIG, musestil()),
            _ => (SVAK, Style::new()),
        };
        buf.set_style(Rect::new(f.fra, rad, f.til - f.fra, 1), lapp);
        buf.set_string(f.fra + 1, rad, &f.ikon.tegn, lapp.patch(f.ikon.stil));
        buf.set_string(f.fra + 1 + f.ikon.bredde, rad, &f.navn, lapp.patch(tekst));
        if f.zoomet {
            let x = f.fra + 1 + f.ikon.bredde + f.navn.chars().count() as u16;
            buf.set_string(x, rad, ZOOM, lapp.patch(tekst));
        }
        if pekt || f.vist {
            let kryss = match på_kryss {
                true => Style::new().fg(ORANSJE).add_modifier(Modifier::BOLD),
                false => SVAK,
            };
            buf.set_string(f.lukk(), rad, "×", lapp.patch(kryss));
        }
    }
    if let Some(pluss) = ny_fane(app) {
        let stil = if mus == Some(&Mål::NyFane) { VANLIG } else { SVAK };
        buf.set_string(pluss.start, rad, " + ", stil);
    }
    let lukk = lukk_nerdr(app);
    let stil = if mus == Some(&Mål::Knapp(Knapp::Avslutt)) { musestil().patch(VANLIG) } else { SVAK };
    buf.set_string(lukk.start, rad, LUKK_NERDR, stil);
}

/// Vinduet til fanen i kolonne `x`, med luften etter den.
pub(super) fn fane_på(app: &App, x: u16) -> Option<String> {
    synlige_faner(app).into_iter().find(|f| (f.fra..=f.til).contains(&x)).map(|f| f.vindu)
}

/// Vinduet til fanen med `×` i kolonne `x`. Rutene på hver side treffer
/// også, så den er lettere å treffe.
pub(super) fn lukk_fane_på(app: &App, x: u16) -> Option<String> {
    synlige_faner(app).into_iter().find(|f| f.lukk().abs_diff(x) <= 1).map(|f| f.vindu)
}

pub(super) const LUKK_NERDR: &str = "lukk nerdr";

/// Høyrestilt på faneraden.
pub(super) fn lukk_nerdr(app: &App) -> Range<u16> {
    let slutt = fanerom(app).end.saturating_sub(1);
    slutt.saturating_sub(LUKK_NERDR.len() as u16)..slutt
}

pub(super) fn lukk_nerdr_på(app: &App, x: u16) -> bool {
    lukk_nerdr(app).contains(&x)
}

/// Fanene, og `+` etter dem, står fram til lukk nerdr.
pub(super) fn fanerom_til_fanene(app: &App) -> Range<u16> {
    fanerom(app).start..lukk_nerdr(app).start.saturating_sub(1)
}

/// `+` etter fanene, når et vindu vises og det er plass til den.
pub(super) fn ny_fane(app: &App) -> Option<Range<u16>> {
    app.valgt.as_ref()?;
    let rom = fanerom_til_fanene(app);
    let fra = faner(app).last().map_or(rom.start, |f| f.til);
    (fra + 3 <= rom.end).then_some(fra..fra + 3)
}

pub(super) fn ny_fane_på(app: &App, x: u16) -> bool {
    ny_fane(app).is_some_and(|r| r.contains(&x))
}
