//! «Ny»-menyen: alt nerdr kan lage, bak én tast og én knapp, så du ikke må
//! kunne tmux for å få en ny Claude, terminal eller et worktree.

use std::path::PathBuf;

use nerdr_kjerne::start;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Valg {
    Worktree,
    Terminal,
    SideOmSide,
    OverOgUnder,
    Mappe,
    Fortsett,
}

impl Valg {
    pub fn tast(self) -> char {
        match self {
            Valg::Worktree => 'w',
            Valg::Terminal => 't',
            Valg::SideOmSide => '%',
            Valg::OverOgUnder => '"',
            Valg::Mappe => 'n',
            Valg::Fortsett => 'r',
        }
    }

    pub fn tekst(self) -> &'static str {
        match self {
            Valg::Worktree => "nytt worktree",
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
    pub git: bool,
}

pub struct Meny {
    pub her: Option<Her>,
    pub valgt: usize,
    /// Navnet på worktreet, når det skrives inn.
    pub navn: Option<String>,
}

impl Meny {
    pub fn ny(her: Option<Her>) -> Meny {
        Meny { her, valgt: 0, navn: None }
    }

    /// Claude-ene først, så en tom rad og terminalene. Uten en pane som vises,
    /// er det ikke noe «her», og et worktree krever git.
    pub fn rader(&self) -> Vec<Option<Valg>> {
        let her = self.her.is_some();
        let git = self.her.as_ref().is_some_and(|h| h.git);
        let mut rader: Vec<Option<Valg>> = [Valg::Mappe, Valg::Worktree, Valg::Fortsett]
            .into_iter()
            .filter(|v| *v != Valg::Worktree || git)
            .map(Some)
            .collect();
        if her {
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

    pub fn skriv(&mut self, tekst: &str) {
        if let Some(navn) = &mut self.navn {
            *navn = start::worktree_navn(&format!("{navn}{tekst}"));
        }
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    fn her(git: bool) -> Option<Her> {
        Some(Her { sesjon_id: "$0".into(), pane_id: "%0".into(), mappe: PathBuf::from("/h"), git })
    }

    #[test]
    fn worktree_bare_i_git_og_bare_mapper_uten_pane() {
        assert!(Meny::ny(her(true)).valg().contains(&Valg::Worktree));
        assert!(!Meny::ny(her(false)).valg().contains(&Valg::Worktree));
        assert_eq!(Meny::ny(None).valg(), [Valg::Mappe, Valg::Fortsett]);
    }

    #[test]
    fn navnet_tar_bare_det_skallet_tåler() {
        let mut meny = Meny::ny(her(true));
        meny.skriv("x");
        assert_eq!(meny.navn, None);
        meny.navn = Some(String::new());
        meny.skriv("-fiks; ls");
        assert_eq!(meny.navn.as_deref(), Some("fiks-ls"));
    }
}
