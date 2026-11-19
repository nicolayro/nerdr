//! Det et kort i agentlista står for.

use std::path::{Path, PathBuf};
use std::time::Instant;

use nerdr_kjerne::merke::Merke;
use nerdr_kjerne::register::Claude;

pub enum Agent {
    Claude(Claude),
    /// Oppstarten tar flere sekunder, og så lenge vises den likevel.
    Starter { mappe: PathBuf, startet: Instant },
    /// Et program i en sesjon nerdr har laget, der Claude-en er borte, så det
    /// du har åpnet, ikke blir borte for deg.
    Rest { mappe: PathBuf, programmer: String },
}

impl Agent {
    pub fn er_claude(&self) -> bool {
        !matches!(self, Agent::Rest { .. })
    }

    pub fn claude(&self) -> Option<&Claude> {
        match self {
            Agent::Claude(c) => Some(c),
            _ => None,
        }
    }

    pub fn merke(&self, sett: u64) -> Merke {
        match self {
            Agent::Claude(c) => Merke::fra(c, sett),
            Agent::Starter { .. } => Merke::Starter,
            Agent::Rest { .. } => Merke::Sett,
        }
    }

    pub fn mappe(&self) -> Option<&Path> {
        match self {
            Agent::Claude(c) => c.mappe.as_deref(),
            Agent::Starter { mappe, .. } | Agent::Rest { mappe, .. } => Some(mappe),
        }
    }

    pub fn endret(&self) -> Option<u64> {
        self.claude().map(|c| c.endret)
    }
}

#[cfg(test)]
mod tester {
    use super::*;
    use nerdr_kjerne::register::Tilstand;

    fn claude(mappe: Option<&str>) -> Agent {
        Agent::Claude(Claude { tilstand: Some(Tilstand::Jobber), endret: 5, mappe: mappe.map(PathBuf::from), samtale: None })
    }

    #[test]
    fn merket_følger_typen() {
        let starter = Agent::Starter { mappe: "/a".into(), startet: Instant::now() };
        let rest = Agent::Rest { mappe: "/a".into(), programmer: "vim".into() };
        assert_eq!(starter.merke(0), Merke::Starter);
        assert_eq!(rest.merke(0), Merke::Sett);
        assert_eq!(claude(None).merke(0), Merke::Jobber);
    }

    #[test]
    fn bare_resten_er_ikke_en_claude() {
        let starter = Agent::Starter { mappe: "/a".into(), startet: Instant::now() };
        assert!(starter.er_claude());
        assert!(claude(None).er_claude());
        assert!(!Agent::Rest { mappe: "/a".into(), programmer: String::new() }.er_claude());
    }

    #[test]
    fn mappa_og_tiden_kommer_fra_riktig_sted() {
        assert_eq!(claude(Some("/b")).mappe(), Some(Path::new("/b")));
        assert_eq!(claude(None).mappe(), None);
        assert_eq!(claude(None).endret(), Some(5));
        let starter = Agent::Starter { mappe: "/a".into(), startet: Instant::now() };
        assert_eq!(starter.mappe(), Some(Path::new("/a")));
        assert_eq!(starter.endret(), None);
    }
}
