use std::collections::HashSet;
use std::path::Path;

use ratatui::crossterm::event::Event;

use super::Prøve;
use crate::handlinger::Handlinger;
use crate::popup::Popup;
use crate::utseende::Akse;
use crate::{Hendelse, Lukke, ny};

fn handlinger(p: &Prøve) -> Popup {
    Popup::Handlinger(Handlinger::ny(&p.app.prefiks.navn(), &p.app.bindinger, &p.app.utseende))
}

fn velger() -> Popup {
    Popup::Velger(ny::Velger::ny(false, Path::new("/finnes/ikke"), &HashSet::new()))
}

fn lukke() -> Popup {
    Popup::Lukke(Lukke { spørsmål: "lukke sesjonen?", hva: "x".into(), kommando: "kill-pane -t '%1'".into() })
}

fn alle(p: &Prøve) -> Vec<Popup> {
    vec![Popup::Hjelp(0), Popup::Utseende(Akse::Kortdesign), lukke(), handlinger(p), velger()]
}

fn lim_inn(p: &mut Prøve, tekst: &str) {
    p.app.håndter(Hendelse::Term(Event::Paste(tekst.into())));
}

#[test]
fn esc_lukker_hver_popup() {
    let mut p = Prøve::ny();
    p.sendt();
    for popup in alle(&p) {
        p.app.popup = Some(popup);
        p.tast("Esc");
        assert!(p.app.popup.is_none());
    }
    assert!(p.sendt().is_empty());
}

#[test]
fn en_tast_som_ikke_lukker_lar_popupen_stå() {
    let mut p = Prøve::ny();
    p.app.popup = Some(Popup::Utseende(Akse::Kortdesign));
    p.tast("j");
    assert!(matches!(p.app.popup, Some(Popup::Utseende(akse)) if akse != Akse::Kortdesign));

    p.app.popup = Some(handlinger(&p));
    p.tast("z");
    assert!(matches!(&p.app.popup, Some(Popup::Handlinger(h)) if h.felt.søk == "z"));

    p.app.popup = Some(velger());
    p.tast("z");
    assert!(matches!(&p.app.popup, Some(Popup::Velger(v)) if v.søk() == "z"));
}

#[test]
fn hjelpen_som_får_plass_lukkes_av_hver_tast() {
    let mut p = Prøve::ny();
    p.app.popup = Some(Popup::Hjelp(0));
    p.tast("j");
    assert!(p.app.popup.is_none());
}

#[test]
fn lukke_sender_bare_ved_y() {
    let mut p = Prøve::ny();
    p.sendt();
    p.app.popup = Some(lukke());
    p.tast("n");
    assert!(p.app.popup.is_none() && p.sendt().is_empty());
    p.app.popup = Some(lukke());
    p.tast("y");
    assert_eq!(p.sendt(), ["kill-pane -t '%1'"]);
}

#[test]
fn enter_i_handlingene_kjører_valget_og_lukker() {
    let mut p = Prøve::ny();
    p.app.popup = Some(handlinger(&p));
    for tegn in "hjelp".chars() {
        p.tast(&tegn.to_string());
    }
    p.tast("Enter");
    assert!(matches!(p.app.popup, Some(Popup::Hjelp(0))));
}

#[test]
fn enter_i_velgeren_uten_treff_lar_den_stå() {
    let mut p = Prøve::ny();
    p.app.popup = Some(velger());
    p.tast("Enter");
    assert!(matches!(p.app.popup, Some(Popup::Velger(_))));
}

#[test]
fn ctrl_r_bytter_i_velgeren_og_gjør_ingenting_i_handlingene() {
    let mut p = Prøve::ny();
    p.app.popup = Some(velger());
    p.tast("C-r");
    assert!(matches!(&p.app.popup, Some(Popup::Velger(v)) if v.fortsett));

    p.app.popup = Some(handlinger(&p));
    p.tast("C-r");
    assert!(matches!(&p.app.popup, Some(Popup::Handlinger(h)) if h.felt.søk.is_empty()));
}

#[test]
fn innliming_går_til_søkefeltet_og_ellers_ikke_til_panen() {
    let mut p = Prøve::ny();
    p.app.popup = Some(handlinger(&p));
    lim_inn(&mut p, "zoom");
    assert!(matches!(&p.app.popup, Some(Popup::Handlinger(h)) if h.felt.søk == "zoom"));

    for popup in [Popup::Hjelp(0), Popup::Utseende(Akse::Kortdesign), lukke()] {
        p.app.popup = Some(popup);
        lim_inn(&mut p, "ikke til panen");
    }
    assert!(p.sendt().iter().all(|k| !k.starts_with("send-keys") && !k.contains("paste")));
}
