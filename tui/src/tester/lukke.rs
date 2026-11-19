use nerdr_kjerne::register::Tilstand;

use super::Prøve;

fn lukk(paner: Vec<nerdr_kjerne::paner::Pane>, pane: &str) -> Prøve {
    let mut p = Prøve::ny();
    p.paner = paner;
    for pane in &p.paner.clone() {
        if pane.kommando == "claude" {
            p.claude(&pane.id, Tilstand::Klar);
        }
    }
    p.endret();
    let pane = p.paner.iter().find(|x| x.id == pane).unwrap().clone();
    p.sendt();
    p.app.lukk_sesjon(&pane);
    p
}

#[test]
fn flere_claude_i_vinduet_lukker_bare_panen() {
    let p = lukk(vec![Prøve::pane("%1", "@1", "$1"), Prøve::pane("%2", "@1", "$1")], "%1");
    assert_eq!(p.app.lukke.as_ref().unwrap().kommando, "kill-pane -t '%1'");
}

#[test]
fn sesjon_fra_nerdr_lukkes_med_sesjonen() {
    let fra_nerdr = nerdr_kjerne::paner::Pane { fra_nerdr: true, ..Prøve::pane("%1", "@1", "$1") };
    let p = lukk(vec![fra_nerdr], "%1");
    assert_eq!(p.app.lukke.as_ref().unwrap().kommando, "kill-session -t '$1'");
}

#[test]
fn sesjon_du_har_laget_selv_lukker_bare_vinduet() {
    let p = lukk(vec![Prøve::pane("%1", "@1", "$1")], "%1");
    assert_eq!(p.app.lukke.as_ref().unwrap().kommando, "kill-window -t '@1'");
}

#[test]
fn bare_y_sender_kommandoen() {
    let mut p = lukk(vec![Prøve::pane("%1", "@1", "$1")], "%1");
    p.tast("n");
    assert!(p.sendt().is_empty() && p.app.lukke.is_none());

    let pane = p.paner[0].clone();
    p.app.lukk_sesjon(&pane);
    p.tast("y");
    assert_eq!(p.sendt(), ["kill-window -t '@1'"]);
}
