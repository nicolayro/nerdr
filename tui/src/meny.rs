//! «Ny»-menyen: alt nerdr kan lage, bak én tast og én knapp, så du ikke må
//! kunne tmux for å få en ny Claude eller terminal.

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Valg {
    Terminal,
    SideOmSide,
    OverOgUnder,
    Mappe,
    Fortsett,
}

impl Valg {
    pub fn tast(self) -> char {
        match self {
            Valg::Terminal => 't',
            Valg::SideOmSide => '%',
            Valg::OverOgUnder => '"',
            Valg::Mappe => 'n',
            Valg::Fortsett => 'r',
        }
    }

    /// Tasten etter prefikset som gjør det samme uten menyen.
    pub fn prefikstast(self) -> char {
        match self {
            Valg::Terminal => 'c',
            Valg::Mappe => 'f',
            v => v.tast(),
        }
    }

    pub fn tekst(self) -> &'static str {
        match self {
            Valg::Terminal => "ny terminal",
            Valg::SideOmSide => "del side om side",
            Valg::OverOgUnder => "del over og under",
            Valg::Mappe => "ny sesjon",
            Valg::Fortsett => "hent samtale",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Her {
    pub sesjon_id: String,
    pub pane_id: String,
    pub mappe: PathBuf,
}

pub struct Meny {
    pub her: Option<Her>,
    pub valgt: usize,
}

impl Meny {
    pub fn ny(her: Option<Her>) -> Meny {
        Meny { her, valgt: 0 }
    }

    /// Claude-ene først, så en tom rad og terminalene. Uten en pane som vises,
    /// er det ikke noe «her».
    pub fn rader(&self) -> Vec<Option<Valg>> {
        let mut rader = vec![Some(Valg::Mappe), Some(Valg::Fortsett)];
        if self.her.is_some() {
            rader.push(None);
            rader.extend([Valg::Terminal, Valg::SideOmSide, Valg::OverOgUnder].map(Some));
        }
        rader
    }

    pub fn valg(&self) -> Vec<Valg> {
        self.rader().into_iter().flatten().collect()
    }

    pub fn flytt(&mut self, steg: isize) {
        let siste = self.valg().len() as isize - 1;
        self.valgt = (self.valgt as isize + steg).clamp(0, siste) as usize;
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn bare_claude_ene_uten_pane() {
        let her = Some(Her { sesjon_id: "$0".into(), pane_id: "%0".into(), mappe: PathBuf::from("/h") });
        assert!(Meny::ny(her).valg().contains(&Valg::Terminal));
        assert_eq!(Meny::ny(None).valg(), [Valg::Mappe, Valg::Fortsett]);
    }
}
