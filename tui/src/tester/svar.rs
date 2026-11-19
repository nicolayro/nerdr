use nerdr_kjerne::register::Tilstand;

use super::Prøve;

fn med_claude() -> Prøve {
    let mut p = Prøve::ny();
    p.paner = vec![Prøve::pane("%1", "@1", "$1")];
    p.claude("%1", Tilstand::Klar);
    p.endret();
    p.tast("Enter");
    p
}

/// `x`, `y`, markør synlig, mus, alternativ skjerm, historikk, musebevegelse.
fn markør(p: &mut Prøve, felt: &str) {
    p.markør = felt.into();
    p.app.utdatert = true;
    p.app.hent();
    p.svar();
}

#[test]
fn markøren_tolkes() {
    let mut p = med_claude();
    markør(&mut p, "4\t2\t1\t1\t0\t10\t1");
    assert_eq!(p.app.aktiv_pane.markør, Some((4, 2)));
    assert!(p.app.aktiv_pane.mus && p.app.aktiv_pane.bevegelse && !p.app.aktiv_pane.alternativ);
    assert_eq!(p.app.aktiv_pane.historie, 10);

    markør(&mut p, "4\t2\t0\t0\t0\t10\t0");
    assert_eq!(p.app.aktiv_pane.markør, None);
}

#[test]
fn det_du_leser_blir_stående_når_historikken_vokser() {
    let mut p = med_claude();
    markør(&mut p, "0\t0\t1\t0\t0\t10\t0");
    p.app.rull_til(4);
    markør(&mut p, "0\t0\t1\t0\t0\t15\t0");
    assert_eq!(p.app.aktiv_pane.rull, 9);
}

#[test]
fn nederst_blir_nederst_og_alternativ_skjerm_har_ingen_historikk() {
    let mut p = med_claude();
    markør(&mut p, "0\t0\t1\t0\t0\t10\t0");
    markør(&mut p, "0\t0\t1\t0\t0\t15\t0");
    assert_eq!(p.app.aktiv_pane.rull, 0);

    p.app.rull_til(3);
    markør(&mut p, "0\t0\t1\t0\t1\t15\t0");
    assert_eq!(p.app.aktiv_pane.rull, 0);
}

#[test]
fn synkmarkør_delt_over_to_output_teller() {
    let mut p = med_claude();
    p.output("%1", r"tekst\033[?20");
    assert!(!p.app.synk.contains_key("%1"));
    p.output("%1", "26h");
    assert!(p.app.synk.contains_key("%1"));
    p.output("%1", r"\033[?2026l");
    assert!(!p.app.synk.contains_key("%1"));
    assert!(p.app.utdatert);
}

#[test]
fn vinduet_som_vises_lukkes_og_vinduet_til_claude_vises() {
    let mut p = Prøve::ny();
    let ved_siden_av = nerdr_kjerne::paner::Pane { vindu_aktiv: false, ..Prøve::pane("%2", "@2", "$1") };
    p.paner = vec![Prøve::pane("%1", "@1", "$1"), ved_siden_av];
    p.claude("%1", Tilstand::Klar);
    p.endret();
    p.tast("Enter");
    p.app.vis_vindu("@2".into());
    assert_eq!(p.app.vist.as_deref(), Some("@2"));

    p.paner.truncate(1);
    p.endret();
    assert_eq!(p.app.vist.as_deref(), Some("@1"));
}

#[test]
fn kortet_viser_oppgaven_fra_panetittelen() {
    let mut p = Prøve::ny();
    p.paner = vec![nerdr_kjerne::paner::Pane { tittel: "✳ Fiks testene".into(), ..Prøve::pane("%1", "@1", "$1") }];
    p.claude("%1", Tilstand::Klar);
    p.endret();
    assert!(p.skjerm().iter().any(|l| l.contains("Fiks testene")));
}
