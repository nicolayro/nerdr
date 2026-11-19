use nerdr_kjerne::kontroll::Melding;
use nerdr_kjerne::register::Tilstand;

use super::Prøve;
use crate::Hendelse;

fn med_claude() -> Prøve {
    let mut p = Prøve::ny();
    p.paner = vec![Prøve::pane("%1", "@1", "$1")];
    p.claude("%1", Tilstand::Klar);
    p.endret();
    p.tast("Enter");
    p
}

#[test]
fn frakoblet_tmux_kobler_til_igjen_og_velger_sesjonen_på_nytt() {
    let mut p = med_claude();
    p.logg.borrow_mut().ny_tilkobling = Some(2);
    p.sendt();
    p.melding(Melding::Slutt(String::new()));
    assert_eq!(p.app.tilkobling, 2);
    assert!(p.app.ferdig.is_none());
    let sendt = p.sendt();
    assert_eq!(sendt[0], "switch-client -t '$1'");
    assert!(sendt.iter().any(|k| k.starts_with("list-panes")));
}

#[test]
fn uten_ny_tilkobling_er_nerdr_ferdig() {
    let mut p = med_claude();
    p.melding(Melding::Slutt(String::new()));
    assert_eq!(p.app.ferdig.as_deref(), Some("tmux koblet fra"));
}

#[test]
fn meldinger_fra_en_gammel_tilkobling_hoppes_over() {
    let mut p = med_claude();
    p.logg.borrow_mut().ny_tilkobling = Some(2);
    p.melding(Melding::Slutt(String::new()));
    p.sendt();
    p.app.håndter(Hendelse::Tmux(1, Melding::Endret));
    assert!(p.sendt().is_empty());
}
