//! Kjernen i nerdr: tmux control mode, sesjonsoversikt og tilstandsdeteksjon
//! for Claude Code. Uten avhengigheter og uten kunnskap om hvordan det vises,
//! så en annen klient kan bygges på den samme kjernen.

pub mod git;
pub mod kontroll;
pub mod oversikt;
pub mod sgr;
pub mod tilstand;
