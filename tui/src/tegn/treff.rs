//! Hva musen står over. Tegningen og klikket spør begge her, så det du ser
//! under musen, er det klikket treffer.

use super::{
    fane_på, fanerad, i_lista, i_menyboks, i_popupen, i_valgmenyen, knapp_på, liste, lukk_fane_på, lukk_nerdr_på, lukk_x,
    meny_rad_på, ny_fane_på, sortering_på, svar_på, verdi_på,
};
use crate::grupper::{Overskrift, Rad};
use crate::popup::Popup;
use crate::{App, Knapp, Mål};
use ratatui::layout::Rect;

pub fn mål(app: &App, x: u16, y: u16) -> Option<Mål> {
    match &app.popup {
        Some(Popup::Hjelp(_)) => None,
        Some(Popup::Utseende(_)) => verdi_på(app, x, y)
            .map(|(akse, i)| Mål::Verdi(akse, i))
            .or_else(|| i_valgmenyen(app, x, y).then_some(Mål::Popup)),
        Some(Popup::Handlinger(_) | Popup::Velger(_)) => {
            meny_rad_på(app, x, y).map(Mål::Rad).or_else(|| i_menyboks(app, x, y).then_some(Mål::Popup))
        }
        Some(Popup::Lukke(_)) => svar_på(app, x, y).map(Mål::Svar).or_else(|| i_popupen(app, x, y).then_some(Mål::Popup)),
        None if x + 1 == app.visning.x => Some(Mål::Skille),
        None if x < app.visning.x => i_agentlista(app, x, y),
        None if y < app.visning.y => i_fanene(app, x, y),
        None => Some(i_visningen(app, x, y)),
    }
}

fn i_agentlista(app: &App, x: u16, y: u16) -> Option<Mål> {
    let (x, y) = i_lista(app, x, y)?;
    if let Some(knapp) = knapp_på(app, x, y) {
        return Some(Mål::Knapp(knapp));
    }
    if sortering_på(app, x, y) {
        return Some(Mål::Sortering);
    }
    let (rader, utsnitt) = liste(app);
    let vis = utsnitt.rader();
    match () {
        _ if !utsnitt.ruller => {}
        _ if y + 1 == vis.start => return Some(Mål::Pil(true)),
        _ if y == vis.end => return Some(Mål::Pil(false)),
        _ => {}
    }
    let Some(rad) = utsnitt.i_lista(y) else { return Some(Mål::Liste) };
    let på_kryss = lukk_x(app).is_some_and(|l| l.abs_diff(x) <= 1);
    for p in &rader {
        match &p.rad {
            Rad::Pane(pane, _) if på_kryss && p.y == rad => return Some(Mål::Lukk(pane.id.clone())),
            Rad::Overskrift(Overskrift::Mappe(navn)) | Rad::Brettet(navn, _) if p.y == rad => {
                return Some(Mål::Prosjekt(navn.clone()));
            }
            Rad::Pane(pane, _) if (p.y..p.y + p.høyde).contains(&rad) => return Some(Mål::Kort(pane.id.clone())),
            _ => {}
        }
    }
    Some(Mål::Liste)
}

fn i_fanene(app: &App, x: u16, y: u16) -> Option<Mål> {
    if y != fanerad(app) {
        return None;
    }
    if lukk_nerdr_på(app, x) {
        return Some(Mål::Knapp(Knapp::Avslutt));
    }
    if let Some(vindu) = lukk_fane_på(app, x) {
        return Some(Mål::LukkFane(vindu));
    }
    let ny = ny_fane_på(app, x).then_some(Mål::NyFane);
    fane_på(app, x).map(Mål::Fane).or(ny)
}

fn i_visningen(app: &App, x: u16, y: u16) -> Mål {
    let inni = |r: Rect| (r.left()..r.right()).contains(&x) && (r.top()..r.bottom()).contains(&y);
    match app.bokser().into_iter().find(|b| inni(b.ramme.unwrap_or(b.innhold))) {
        Some(boks) => Mål::Pane(boks.pane),
        None => Mål::Visning,
    }
}
