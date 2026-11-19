use std::time::Duration;

use nerdr_kjerne::merke::Merke;
use nerdr_kjerne::register::Tilstand;

use super::Prøve;
use crate::agent::Agent;
use crate::Svar;

/// `%1` er en Claude nerdr nettopp har startet, og som ikke er i registeret.
fn starter(startet: std::time::Instant) -> Prøve {
    let mut p = Prøve::ny();
    p.paner = vec![Prøve::pane("%1", "@1", "$1")];
    p.app.agenter.insert("%1".into(), Agent::Starter { mappe: "/finnes/ikke".into(), startet });
    p.endret();
    p
}

fn merke(p: &Prøve) -> Option<Merke> {
    p.app.merke(&p.paner[0])
}

#[test]
fn en_som_starter_har_kort_til_registeret_har_den() {
    let mut p = starter(std::time::Instant::now());
    assert_eq!(merke(&p), Some(Merke::Starter));
    assert!(p.skjerm().iter().any(|l| l.contains("starter …")));

    p.claude("%1", Tilstand::Jobber);
    p.endret();
    assert_eq!(merke(&p), Some(Merke::Jobber));
    assert!(!p.skjerm().iter().any(|l| l.contains("starter …")));
}

#[test]
fn en_som_starter_og_aldri_kommer_i_registeret_forsvinner() {
    let mut p = Prøve::ny();
    p.app.starter_maks = Duration::ZERO;
    p.paner = vec![Prøve::pane("%1", "@1", "$1")];
    p.app.agenter.insert("%1".into(), Agent::Starter { mappe: "/finnes/ikke".into(), startet: std::time::Instant::now() });
    p.endret();
    assert_eq!(merke(&p), None);
    assert!(p.app.agent_paner.is_empty());
}

#[test]
fn en_som_starter_står_igjen_når_lista_feiler() {
    let mut p = starter(std::time::Instant::now());
    p.melding(nerdr_kjerne::kontroll::Melding::Endret);
    assert!(matches!(p.app.kø.front(), Some(Svar::Paner)));
    p.melding(nerdr_kjerne::kontroll::Melding::Svar { ok: false, linjer: Vec::new() });
    assert!(p.app.agenter.contains_key("%1"));
    p.endret();
    assert_eq!(merke(&p), Some(Merke::Starter));
}
