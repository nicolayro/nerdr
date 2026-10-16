//! Kjernen i nerdr: tmux control mode, panene på serveren og tilstanden til
//! Claude-sesjonene. Vet ikke hvordan noe vises, så en annen klient kan bygges
//! på den samme kjernen.

pub mod git;
pub mod kontroll;
pub mod paner;
pub mod register;
pub mod samtaler;
pub mod sgr;
