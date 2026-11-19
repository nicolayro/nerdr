use std::collections::HashSet;

use nerdr_kjerne::paner::Pane;
use nerdr_kjerne::register::Tilstand;
use ratatui::crossterm::event::{MouseButton, MouseEventKind};

use super::Prøve;
use crate::popup::Popup;
use crate::utseende::Akse;
use crate::Fokus;
use crate::tegn::{self, Mål};

/// `%1` er klar og `%2` jobber, hver i sitt vindu.
fn to_claude() -> Prøve {
    let mut p = Prøve::ny();
    let annet = Pane { vindunavn: "annet".into(), ..Prøve::pane("%2", "@2", "$2") };
    p.paner = vec![Prøve::pane("%1", "@1", "$1"), annet];
    p.claude("%1", Tilstand::Klar);
    p.claude("%2", Tilstand::Jobber);
    p.endret();
    p
}

fn valgt(p: &Prøve) -> Option<&str> {
    p.app.valgt.as_ref().map(|v| v.id.as_str())
}

#[test]
fn et_klikk_på_et_kort_går_inn_i_claude_en() {
    let mut p = to_claude();
    let (x, y) = p.finn("s1:0");
    p.klikk(x, y);
    assert_eq!(valgt(&p), Some("%1"));
    assert!(p.app.fokus == Fokus::Pane);

    let (x, y) = p.finn("s2:0");
    p.klikk(x, y);
    assert_eq!(valgt(&p), Some("%2"));
}

#[test]
fn et_klikk_på_kryssen_som_tegnes_under_musen_spør_om_å_lukke_den_panen() {
    let mut p = to_claude();
    let (x, y) = p.finn("s2:0");
    p.mus(MouseEventKind::Moved, x, y);
    // Kortet starter raden over navnet, og fanen har sin egen ×.
    let kryss_y = y - 1;
    let kryss_x = p.skjerm()[kryss_y as usize].chars().position(|c| c == '×').expect("kryssen er tegnet") as u16;
    p.klikk(kryss_x, kryss_y);
    assert_eq!(p.app.lukke().map(|l| l.kommando.as_str()), Some("kill-window -t '@2'"));
}

#[test]
fn ny_sesjon_åpner_velgeren() {
    let mut p = to_claude();
    let (x, y) = p.finn("ny sesjon");
    p.klikk(x, y);
    assert!(matches!(p.app.popup, Some(Popup::Velger(_))));
}

#[test]
fn sorteringen_bytter_orden() {
    let mut p = to_claude();
    let før = p.app.utseende.indeks(Akse::Orden);
    let (x, y) = p.finn("prioritert");
    p.klikk(x, y);
    assert_ne!(p.app.utseende.indeks(Akse::Orden), før);
}

#[test]
fn fanen_viser_vinduet_og_midtknappen_lukker_det() {
    let mut p = to_claude();
    let (x, y) = p.finn("annet");
    p.klikk(x, y);
    assert_eq!(valgt(&p), Some("%2"));

    p.sendt();
    p.mus(MouseEventKind::Down(MouseButton::Middle), x, y);
    assert_eq!(p.sendt(), ["kill-window -t '@2'"]);
}

#[test]
fn et_klikk_inni_en_popup_lar_den_stå_og_utenfor_lukker_den() {
    let mut p = to_claude();
    p.app.popup = Some(Popup::Utseende(Akse::Kortdesign));
    let (x, y) = p.finn("innstillinger");
    p.klikk(x, y);
    assert!(p.app.popup.is_some());
    p.klikk(0, 39);
    assert!(p.app.popup.is_none());
}

#[test]
fn en_fane_som_ikke_får_plass_kan_ikke_treffes() {
    let mut p = Prøve::ny();
    p.paner = (1..=8)
        .map(|i| Pane { vindunavn: format!("vindu-{i}-xxxxxxxxxxxxx"), ..Prøve::pane(&format!("%{i}"), &format!("@{i}"), "$1") })
        .collect();
    for i in 1..=8 {
        p.claude(&format!("%{i}"), Tilstand::Klar);
    }
    p.endret();
    let skjerm = p.skjerm();
    assert!(!skjerm[0].contains("vindu-8"), "den siste fanen får ikke plass");
    let truffet: HashSet<String> = (0..120)
        .filter_map(|x| match tegn::treff::mål(&p.app, x, 0) {
            Some(Mål::Fane(v) | Mål::LukkFane(v)) => Some(v),
            _ => None,
        })
        .collect();
    assert!(!truffet.is_empty() && !truffet.contains("@8"), "{truffet:?}");
}

#[test]
fn hjulet_ruller_lista_også_over_ny_sesjon() {
    let mut p = Prøve::ny();
    p.paner = (1..=30).map(|i| Prøve::pane(&format!("%{i}"), &format!("@{i}"), &format!("${i}"))).collect();
    for i in 1..=30 {
        p.claude(&format!("%{i}"), Tilstand::Klar);
    }
    p.endret();
    let (x, y) = p.finn("ny sesjon");
    p.skjerm();
    p.mus(MouseEventKind::ScrollDown, x, y);
    assert!(p.app.liste_rull > 0);
}
