//! Kjernen i nerdr: tmux control mode og panene på serveren. Uten
//! avhengigheter og uten kunnskap om hvordan det vises, så en annen klient kan
//! bygges på den samme kjernen.

pub mod kontroll;
pub mod paner;
pub mod sgr;
