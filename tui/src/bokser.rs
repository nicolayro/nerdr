//! Hvor panene i vinduet tegnes, med bokser eller med linjer.
//!
//! Med bokser står hver pane i en boks med avrundede hjørner, med luft inni og
//! mellom boksene. tmux lar én kolonne eller rad stå tom mellom panene og
//! ingenting ytterst, så vinduet må være så mye mindre enn visningen som
//! `ekstra` sier. Det avhenger bare av hvordan vinduet er delt, ikke av
//! størrelsen, så tmux får ny størrelse bare når panene deles eller lukkes.
//!
//! Med linjer står panene der tmux har plassert dem, og strekene tegnes i
//! rutene tmux lar stå tomme.

use nerdr_kjerne::oppsett::{Celle, Innhold};
use ratatui::layout::Rect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rammer {
    Bokser,
    Linjer,
}

/// Kolonner mellom rammen og innholdet, til venstre og høyre.
const LUFT_INNI: u16 = 1;
/// Kolonner og rader mellom to bokser. tmux lar én stå tom, så med 0 står
/// boksene tettere enn panene i tmux.
const MELLOM_X: u16 = 1;
const MELLOM_Y: u16 = 0;
/// Kolonner mellom boksene og kantene av visningen.
const KANT: u16 = 1;
const BOKS_X: u16 = 2 + 2 * LUFT_INNI;
const BOKS_Y: u16 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct Boks {
    pub pane: String,
    /// Uten ramme med linjer.
    pub ramme: Option<Rect>,
    pub innhold: Rect,
}

/// Kolonnene og radene rammene trenger i tillegg til vinduet.
pub fn ekstra(oppsett: &Celle, rammer: Rammer) -> (u16, u16) {
    match rammer {
        Rammer::Linjer => (0, 0),
        Rammer::Bokser => {
            let (x, y) = ekstra_i(oppsett);
            (x + 2 * KANT, y)
        }
    }
}

fn ekstra_i(c: &Celle) -> (u16, u16) {
    let barn = |celler: &[Celle]| celler.iter().map(ekstra_i).collect::<Vec<_>>();
    // Boksene trenger `luft` mellom seg der tmux har én tom.
    let rekke = |sum: u16, celler: &[Celle], luft: u16| {
        let mellomrom = (celler.len() as u16).saturating_sub(1);
        (sum + mellomrom * luft).saturating_sub(mellomrom)
    };
    match &c.innhold {
        Innhold::Pane(_) => (BOKS_X, BOKS_Y),
        Innhold::SideOmSide(celler) => {
            let e = barn(celler);
            let x = rekke(e.iter().map(|e| e.0).sum(), celler, MELLOM_X);
            (x, e.iter().map(|e| e.1).max().unwrap_or(0))
        }
        Innhold::OverHverandre(celler) => {
            let e = barn(celler);
            let y = rekke(e.iter().map(|e| e.1).sum(), celler, MELLOM_Y);
            (e.iter().map(|e| e.0).max().unwrap_or(0), y)
        }
    }
}

pub fn plasser(oppsett: &Celle, område: Rect, rammer: Rammer) -> Vec<Boks> {
    let mut ut = Vec::new();
    match rammer {
        Rammer::Linjer => plasser_linjer(oppsett, område, &mut ut),
        Rammer::Bokser => {
            let r = Rect::new(område.x + KANT, område.y, område.width.saturating_sub(2 * KANT), område.height);
            plasser_bokser(oppsett, r, &mut ut);
        }
    }
    ut
}

fn plasser_linjer(c: &Celle, område: Rect, ut: &mut Vec<Boks>) {
    match &c.innhold {
        Innhold::Pane(pane) => {
            let innhold = Rect::new(område.x + c.x, område.y + c.y, c.bredde, c.høyde).intersection(område);
            ut.push(Boks { pane: pane.clone(), ramme: None, innhold });
        }
        Innhold::SideOmSide(celler) | Innhold::OverHverandre(celler) => {
            for b in celler {
                plasser_linjer(b, område, ut);
            }
        }
    }
}

/// En celle får plassen tmux har gitt den, pluss det boksene i den trenger.
/// Den siste cellen i en rekke får resten, så kantene står likt der en
/// nabo har flere bokser i seg.
fn plasser_bokser(c: &Celle, r: Rect, ut: &mut Vec<Boks>) {
    match &c.innhold {
        Innhold::Pane(pane) => {
            let (x, y) = (r.x + 1 + LUFT_INNI, r.y + 1);
            let indre = Rect::new(x, y, r.width.saturating_sub(BOKS_X), r.height.saturating_sub(BOKS_Y));
            let innhold = Rect::new(x, y, c.bredde, c.høyde).intersection(indre);
            ut.push(Boks { pane: pane.clone(), ramme: Some(r), innhold });
        }
        Innhold::SideOmSide(celler) => {
            let mut x = r.x;
            for (i, b) in celler.iter().enumerate() {
                let bredde = match i + 1 == celler.len() {
                    true => r.right().saturating_sub(x),
                    false => (b.bredde + ekstra_i(b).0).min(r.right().saturating_sub(x)),
                };
                plasser_bokser(b, Rect::new(x, r.y, bredde, r.height), ut);
                x = (x + bredde + MELLOM_X).min(r.right());
            }
        }
        Innhold::OverHverandre(celler) => {
            let mut y = r.y;
            for (i, b) in celler.iter().enumerate() {
                let høyde = match i + 1 == celler.len() {
                    true => r.bottom().saturating_sub(y),
                    false => (b.høyde + ekstra_i(b).1).min(r.bottom().saturating_sub(y)),
                };
                plasser_bokser(b, Rect::new(r.x, y, r.width, høyde), ut);
                y = (y + høyde + MELLOM_Y).min(r.bottom());
            }
        }
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    fn pane(id: &str, bredde: u16, høyde: u16, x: u16, y: u16) -> Celle {
        Celle { bredde, høyde, x, y, innhold: Innhold::Pane(id.into()) }
    }

    #[test]
    fn én_pane_får_boks_med_luft() {
        // Visningen er 80x20, og tmux har fått 74x18.
        let p = pane("%1", 74, 18, 0, 0);
        assert_eq!(ekstra(&p, Rammer::Bokser), (6, 2));
        let bokser = plasser(&p, Rect::new(36, 0, 80, 20), Rammer::Bokser);
        assert_eq!(bokser[0].ramme, Some(Rect::new(37, 0, 78, 20)));
        assert_eq!(bokser[0].innhold, Rect::new(39, 1, 74, 18));
    }

    #[test]
    fn med_linjer_står_panene_der_tmux_har_dem() {
        let c = Celle {
            bredde: 80,
            høyde: 20,
            x: 0,
            y: 0,
            innhold: Innhold::SideOmSide(vec![pane("%1", 40, 20, 0, 0), pane("%2", 39, 20, 41, 0)]),
        };
        assert_eq!(ekstra(&c, Rammer::Linjer), (0, 0));
        let bokser = plasser(&c, Rect::new(36, 0, 80, 20), Rammer::Linjer);
        assert_eq!(bokser[1], Boks { pane: "%2".into(), ramme: None, innhold: Rect::new(77, 0, 39, 20) });
    }

    #[test]
    fn side_om_side_med_luft_mellom() {
        // Visningen er 80x20, og tmux har fått 70x18: 34 + 1 + 35.
        let c = Celle {
            bredde: 70,
            høyde: 18,
            x: 0,
            y: 0,
            innhold: Innhold::SideOmSide(vec![pane("%1", 34, 18, 0, 0), pane("%2", 35, 18, 35, 0)]),
        };
        assert_eq!(ekstra(&c, Rammer::Bokser), (10, 2));
        let bokser = plasser(&c, Rect::new(0, 0, 80, 20), Rammer::Bokser);
        assert_eq!(bokser[0].ramme, Some(Rect::new(1, 0, 38, 20)));
        assert_eq!(bokser[0].innhold, Rect::new(3, 1, 34, 18));
        assert_eq!(bokser[1].ramme, Some(Rect::new(40, 0, 39, 20)));
        assert_eq!(bokser[1].innhold, Rect::new(42, 1, 35, 18));
    }

    #[test]
    fn nabo_med_færre_bokser_fyller_ut_til_samme_kant() {
        // Venstre har to bokser rett over hverandre og trenger én rad mer enn
        // høyre.
        let venstre = Celle {
            bredde: 30,
            høyde: 16,
            x: 0,
            y: 0,
            innhold: Innhold::OverHverandre(vec![pane("%1", 30, 8, 0, 0), pane("%2", 30, 7, 0, 9)]),
        };
        let c = Celle {
            bredde: 61,
            høyde: 16,
            x: 0,
            y: 0,
            innhold: Innhold::SideOmSide(vec![venstre, pane("%3", 30, 16, 31, 0)]),
        };
        assert_eq!(ekstra(&c, Rammer::Bokser), (10, 3));
        let bokser = plasser(&c, Rect::new(0, 0, 71, 19), Rammer::Bokser);
        assert_eq!(bokser[1].ramme, Some(Rect::new(1, 10, 34, 9)));
        assert_eq!(bokser[1].innhold, Rect::new(3, 11, 30, 7));
        assert_eq!(bokser[2].ramme, Some(Rect::new(36, 0, 34, 19)));
        assert_eq!(bokser[2].innhold, Rect::new(38, 1, 30, 16));
    }
}
