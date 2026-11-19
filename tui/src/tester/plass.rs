use nerdr_kjerne::register::Tilstand;

use super::Prøve;
use crate::grupper::Del;

/// To Claude-er, `%1` jobber og `%2` er klar.
fn to() -> Prøve {
    let mut p = Prøve::ny();
    p.paner = vec![Prøve::pane("%1", "@1", "$1"), Prøve::pane("%2", "@2", "$2")];
    p.claude("%1", Tilstand::Jobber);
    p.claude("%2", Tilstand::Klar);
    p.endret();
    p
}

fn del(p: &Prøve, pane: &str) -> Del {
    let pane = p.paner.iter().find(|x| x.id == pane).unwrap();
    p.app.plass(pane).unwrap().del
}

#[test]
fn kortet_du_er_inne_i_blir_ikke_ledig_før_du_er_ute() {
    let mut p = to();
    p.tast("Enter");
    p.claude("%1", Tilstand::Klar);
    p.endret();
    assert_eq!(del(&p, "%1"), Del::Aktive);

    p.tast("C-b");
    p.tast("w");
    p.endret();
    assert_eq!(del(&p, "%1"), Del::Ledige);
}

#[test]
fn forlatt_holder_plassen_til_registeret_er_lest_på_nytt() {
    let mut p = to();
    p.tast("Enter");
    p.tast("C-b");
    p.tast("w");
    assert!(p.app.forlatt.is_some());
    p.endret();
    assert!(p.app.forlatt.is_none());
}

#[test]
fn ny_claude_regnes_som_sett_og_husket_i_tmux() {
    let mut p = to();
    assert!(p.sendt().iter().any(|k| k.starts_with("set-option -p -t '%1' @nerdr-sett ")));
    assert_eq!(del(&p, "%2"), Del::Ledige);
}

#[test]
fn størst_av_tiden_i_nerdr_og_i_tmux_vinner() {
    let mut p = to();
    let nå = p.app.sett["%1"];
    p.paner[0].sett = Some(nå + 1000);
    p.endret();
    assert_eq!(p.app.sett["%1"], nå + 1000);
    p.paner[0].sett = Some(5);
    p.endret();
    assert_eq!(p.app.sett["%1"], nå + 1000);
}
