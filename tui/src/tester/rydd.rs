use std::time::Duration;

use nerdr_kjerne::paner::Pane;
use nerdr_kjerne::register::Tilstand;

use super::{Prøve, for_lenge_siden};
use crate::UTEN_CLAUDE_MAKS;

/// En sesjon fra nerdr som har mistet Claude-en for en stund siden.
fn uten_claude(andre_paner: Vec<Pane>) -> Prøve {
    let mut p = Prøve::ny();
    let skall = Pane { kommando: "zsh".into(), fra_nerdr: true, ..Prøve::pane("%1", "@1", "$1") };
    p.paner = std::iter::once(skall).chain(andre_paner).collect();
    p.claude("%1", Tilstand::Klar);
    p.endret();
    p.uten_claude("%1");
    p.endret();
    p
}

fn lenge_nok(p: &mut Prøve) {
    let tid = for_lenge_siden(UTEN_CLAUDE_MAKS + Duration::from_secs(1));
    p.app.uten_claude.insert("$1".into(), tid);
    p.sendt();
    p.endret();
}

#[test]
fn sesjonen_får_tid_til_å_starte_claude() {
    let mut p = uten_claude(vec![]);
    assert!(!p.sendt().iter().any(|k| k.starts_with("kill-session")));
}

#[test]
fn bare_skall_igjen_lukker_sesjonen() {
    let mut p = uten_claude(vec![]);
    lenge_nok(&mut p);
    assert!(p.sendt().contains(&"kill-session -t '$1'".to_string()));
}

#[test]
fn et_program_igjen_blir_et_kort() {
    let nvim = Pane { kommando: "nvim".into(), fra_nerdr: true, ..Prøve::pane("%2", "@2", "$1") };
    let mut p = uten_claude(vec![nvim]);
    lenge_nok(&mut p);
    assert!(!p.sendt().iter().any(|k| k.starts_with("kill-session")));
    assert_eq!(p.app.rester.get("%2").map(String::as_str), Some("nvim"));
    assert!(p.app.claude_paner.iter().any(|x| x.id == "%2"));
}
