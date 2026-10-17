//! Hva som venter på deg: det Claude melder selv, sammen med når du sist var
//! inne i panen.

use crate::register::{Claude, Tilstand};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Merke {
    Spør,
    /// Ferdig med noe du ikke har sett.
    Ulest,
    Jobber,
    Sett,
    /// Startet av nerdr og ikke i registeret ennå.
    Starter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Gruppe {
    /// Venter på deg.
    Venter,
    Jobber,
    Ferdig,
}

impl Merke {
    /// `sett` er da du sist var inne i panen, i millisekunder siden 1970.
    pub fn fra(claude: &Claude, sett: u64) -> Merke {
        match claude.tilstand {
            Tilstand::Spør => Merke::Spør,
            Tilstand::Jobber => Merke::Jobber,
            Tilstand::Klar if claude.endret > sett => Merke::Ulest,
            Tilstand::Klar => Merke::Sett,
        }
    }

    pub fn gruppe(self) -> Gruppe {
        match self {
            Merke::Spør | Merke::Ulest => Gruppe::Venter,
            Merke::Jobber | Merke::Starter => Gruppe::Jobber,
            Merke::Sett => Gruppe::Ferdig,
        }
    }

    /// Står i JSON fra `nerdr liste`, og endres ikke.
    pub fn navn(self) -> &'static str {
        match self {
            Merke::Spør => "spør",
            Merke::Ulest => "ulest",
            Merke::Jobber => "jobber",
            Merke::Sett => "sett",
            Merke::Starter => "starter",
        }
    }
}

impl Gruppe {
    /// Står i JSON fra `nerdr liste`, og endres ikke.
    pub fn navn(self) -> &'static str {
        match self {
            Gruppe::Venter => "venter",
            Gruppe::Jobber => "jobber",
            Gruppe::Ferdig => "ferdig",
        }
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    fn claude(tilstand: Tilstand, endret: u64) -> Claude {
        Claude { tilstand, endret, mappe: None, samtale: None }
    }

    #[test]
    fn ferdig_etter_at_du_var_inne_venter_på_deg() {
        assert_eq!(Merke::fra(&claude(Tilstand::Klar, 20), 10), Merke::Ulest);
        assert_eq!(Merke::fra(&claude(Tilstand::Klar, 10), 20), Merke::Sett);
        assert_eq!(Merke::fra(&claude(Tilstand::Spør, 10), 20), Merke::Spør);
        assert_eq!(Merke::Ulest.gruppe(), Gruppe::Venter);
    }
}
