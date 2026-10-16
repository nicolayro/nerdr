//! Hvor panene i et delt vindu tegnes: hver i en boks med avrundede hjørner
//! og luft mellom boksene.
//!
//! tmux lar én kolonne eller rad stå tom mellom panene og ingenting ytterst.
//! En boks trenger én rute ekstra på hver side, og den tomme ruta blir luften
//! mellom to bokser. Vinduet må derfor være så mye mindre enn visningen som
//! `ekstra` sier. Det avhenger bare av hvordan vinduet er delt, ikke av
//! størrelsen, så tmux får ny størrelse bare når panene deles eller lukkes.

use nerdr_kjerne::oppsett::{Celle, Innhold};
use ratatui::layout::Rect;

#[derive(Clone, Debug, PartialEq)]
pub struct Boks {
    pub pane: String,
    /// Uten ramme når vinduet bare har én pane.
    pub ramme: Option<Rect>,
    pub innhold: Rect,
}

/// Kolonnene og radene boksene trenger i tillegg til vinduet.
pub fn ekstra(oppsett: &Celle) -> (u16, u16) {
    match &oppsett.innhold {
        Innhold::Pane(_) => (0, 0),
        _ => ekstra_i(oppsett),
    }
}

fn ekstra_i(c: &Celle) -> (u16, u16) {
    let barn = |celler: &[Celle]| celler.iter().map(ekstra_i).collect::<Vec<_>>();
    match &c.innhold {
        Innhold::Pane(_) => (2, 2),
        Innhold::SideOmSide(celler) => {
            let e = barn(celler);
            (e.iter().map(|e| e.0).sum(), e.iter().map(|e| e.1).max().unwrap_or(0))
        }
        Innhold::OverHverandre(celler) => {
            let e = barn(celler);
            (e.iter().map(|e| e.0).max().unwrap_or(0), e.iter().map(|e| e.1).sum())
        }
    }
}

pub fn plasser(oppsett: &Celle, område: Rect) -> Vec<Boks> {
    let mut ut = Vec::new();
    match &oppsett.innhold {
        Innhold::Pane(pane) => {
            let innhold = Rect::new(område.x, område.y, oppsett.bredde, oppsett.høyde).intersection(område);
            ut.push(Boks { pane: pane.clone(), ramme: None, innhold });
        }
        _ => plasser_i(oppsett, område, &mut ut),
    }
    ut
}

/// En celle får plassen tmux har gitt den, pluss det boksene i den trenger.
/// Den siste cellen i en rekke får resten, så kantene står likt der en
/// nabo har flere bokser i seg.
fn plasser_i(c: &Celle, r: Rect, ut: &mut Vec<Boks>) {
    match &c.innhold {
        Innhold::Pane(pane) => {
            let indre = Rect::new(r.x + 1, r.y + 1, r.width.saturating_sub(2), r.height.saturating_sub(2));
            let innhold = Rect::new(indre.x, indre.y, c.bredde, c.høyde).intersection(indre);
            ut.push(Boks { pane: pane.clone(), ramme: Some(r), innhold });
        }
        Innhold::SideOmSide(celler) => {
            let mut x = r.x;
            for (i, b) in celler.iter().enumerate() {
                let bredde = match i + 1 == celler.len() {
                    true => r.right().saturating_sub(x),
                    false => (b.bredde + ekstra_i(b).0).min(r.right().saturating_sub(x)),
                };
                plasser_i(b, Rect::new(x, r.y, bredde, r.height), ut);
                x = (x + bredde + 1).min(r.right());
            }
        }
        Innhold::OverHverandre(celler) => {
            let mut y = r.y;
            for (i, b) in celler.iter().enumerate() {
                let høyde = match i + 1 == celler.len() {
                    true => r.bottom().saturating_sub(y),
                    false => (b.høyde + ekstra_i(b).1).min(r.bottom().saturating_sub(y)),
                };
                plasser_i(b, Rect::new(r.x, y, r.width, høyde), ut);
                y = (y + høyde + 1).min(r.bottom());
            }
        }
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    fn pane(id: &str, bredde: u16, høyde: u16) -> Celle {
        Celle { bredde, høyde, innhold: Innhold::Pane(id.into()) }
    }

    #[test]
    fn én_pane_fyller_visningen_uten_ramme() {
        let bokser = plasser(&pane("%1", 80, 20), Rect::new(36, 0, 80, 20));
        assert_eq!(bokser, [Boks { pane: "%1".into(), ramme: None, innhold: Rect::new(36, 0, 80, 20) }]);
        assert_eq!(ekstra(&pane("%1", 80, 20)), (0, 0));
    }

    #[test]
    fn side_om_side_med_luft_mellom() {
        // Visningen er 80x20, og tmux har fått 76x18: 37 + 1 + 38.
        let c = Celle { bredde: 76, høyde: 18, innhold: Innhold::SideOmSide(vec![pane("%1", 37, 18), pane("%2", 38, 18)]) };
        assert_eq!(ekstra(&c), (4, 2));
        let bokser = plasser(&c, Rect::new(0, 0, 80, 20));
        assert_eq!(bokser[0].ramme, Some(Rect::new(0, 0, 39, 20)));
        assert_eq!(bokser[0].innhold, Rect::new(1, 1, 37, 18));
        assert_eq!(bokser[1].ramme, Some(Rect::new(40, 0, 40, 20)));
        assert_eq!(bokser[1].innhold, Rect::new(41, 1, 38, 18));
    }

    #[test]
    fn nabo_med_færre_bokser_fyller_ut_til_samme_kant() {
        // Venstre har to bokser over hverandre og trenger to rader mer enn høyre.
        let venstre = Celle { bredde: 30, høyde: 16, innhold: Innhold::OverHverandre(vec![pane("%1", 30, 8), pane("%2", 30, 7)]) };
        let c = Celle { bredde: 61, høyde: 16, innhold: Innhold::SideOmSide(vec![venstre, pane("%3", 30, 16)]) };
        assert_eq!(ekstra(&c), (4, 4));
        let bokser = plasser(&c, Rect::new(0, 0, 65, 20));
        assert_eq!(bokser[2].ramme, Some(Rect::new(33, 0, 32, 20)));
        assert_eq!(bokser[2].innhold, Rect::new(34, 1, 30, 16));
    }
}
