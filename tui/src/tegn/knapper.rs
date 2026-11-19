//! Knappene i lista og hintet nederst.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::ops::Range;
use crate::App;
use crate::bokser::Rammer;
use crate::utseende;
use super::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Knapp {
    Ny,
    Hjelp,
    Avslutt,
    Samtale,
    Terminal,
    Innstillinger,
    Handlinger,
}

/// Når du har trykket prefikset, eller er i kopimodus. Det står nederst, så
/// toppen ikke endrer seg.
pub(super) fn hint(app: &App) -> Option<String> {
    match () {
        _ if app.etter_prefiks => Some(format!(" {} … f ny · w agentlista · ? mer", app.prefiks.navn())),
        _ if app.aktiv_pane.kopimodus.is_some() => Some(" v marker · y kopier · q ut".to_string()),
        _ => None,
    }
}

/// Fra stripa går hintet inn over panen, og derfor tegnes det etter den.
pub(super) fn tegn_hint(buf: &mut Buffer, område: Rect, app: &App) {
    let Some(hint) = hint(app) else { return };
    let f = listeflate(app);
    let ramme = (app.utseende.paner == Rammer::Vindu) as u16;
    let bredde = if stripe(app) { område.width.saturating_sub(f.x + ramme) } else { liste_bredde(app) - 1 };
    let b = bredde as usize;
    buf.set_stringn(f.x, f.bottom().saturating_sub(1), format!("{hint:<b$}"), b, VANLIG);
}

/// Knappene i agentlista, med tasten etter prefikset som gjør det samme.
pub(super) const KNAPPER: [(Knapp, &str, char); 7] = [
    (Knapp::Ny, "ny sesjon", 'f'),
    (Knapp::Hjelp, "? hjelp", '?'),
    (Knapp::Avslutt, LUKK_NERDR, 'd'),
    (Knapp::Samtale, "hent samtale", 'r'),
    (Knapp::Terminal, "ny terminal", 'c'),
    (Knapp::Innstillinger, "innstillinger", 'u'),
    (Knapp::Handlinger, "handlinger", ':'),
];

/// Nederst i agentlista, når de er slått på i innstillingene.
pub(super) const NEDERST: [Knapp; 5] = [Knapp::Ny, Knapp::Samtale, Knapp::Terminal, Knapp::Innstillinger, Knapp::Handlinger];

/// Lista må ha plass til noen kort over knappene nederst.
pub(super) const HØYDE_TIL_KNAPPENE: u16 = 16;

pub(super) fn knapper_nederst(app: &App) -> bool {
    app.utseende.snarveier == utseende::Snarveier::På && !stripe(app) && listeflate(app).height >= HØYDE_TIL_KNAPPENE
}

/// Streken over knappene nederst, og én rad per knapp.
pub(super) fn rader_nederst(app: &App) -> u16 {
    if knapper_nederst(app) { NEDERST.len() as u16 + 1 } else { 0 }
}

/// Et klikk på knappen er det samme som prefikset og denne tasten.
pub fn tasten_til(knapp: Knapp) -> char {
    KNAPPER.iter().find(|k| k.0 == knapp).map_or('?', |k| k.2)
}

/// Plussen og «ny sesjon», med luft til sorteringen.
pub(super) const NY_BREDDE: u16 = 14;

/// Der sorteringen og hjelp står, høyrestilt.
pub(super) fn høyre_kolonne(app: &App) -> u16 {
    sortering(app).map_or(liste_bredde(app) - 1, |(x, _)| x.start)
}

/// Knappene i agentlista, med kolonnene og raden de står på.
pub(super) fn knapper(app: &App) -> Vec<(Knapp, Range<u16>, u16)> {
    if stripe(app) {
        return vec![(Knapp::Ny, 0..3, NY_RAD)];
    }
    let (midt, slutt) = (høyre_kolonne(app), liste_bredde(app) - 1);
    let mut ut = vec![(Knapp::Ny, 0..midt, NY_RAD)];
    if midt < slutt {
        ut.push((Knapp::Hjelp, midt..slutt, 0));
    }
    if knapper_nederst(app) {
        let første = listeflate(app).height - NEDERST.len() as u16;
        ut.extend(NEDERST.iter().zip(første..).map(|(&k, y)| (k, 0..slutt, y)));
    }
    ut
}

pub(super) fn knapp_på(app: &App, x: u16, y: u16) -> Option<Knapp> {
    knapper(app).into_iter().find(|(_, r, rad)| *rad == y && r.contains(&x)).map(|(k, ..)| k)
}
