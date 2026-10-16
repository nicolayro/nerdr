//! Enkeltstående tmux-kommandoer, og kommandoer som linjer for control mode.
//!
//! Kommandoene bygges som argumenter, så de samme kan kjøres direkte eller
//! sendes gjennom control mode.

use std::io;
use std::process::Command;

use crate::kontroll::sitat;

/// `tmux`, med `-L socket` når den er gitt. Uten den, og fra innsiden av tmux,
/// brukes serveren i `$TMUX`, som er den rette for en agent som kjører der.
pub fn kommando(socket: Option<&str>) -> Command {
    let mut tmux = Command::new("tmux");
    if let Some(socket) = socket {
        tmux.args(["-L", socket]);
    }
    tmux
}

/// Kjører én kommando og gir det den skrev ut, eller en feil med det tmux
/// skrev på stderr.
pub fn kjør<S: AsRef<str>>(socket: Option<&str>, argumenter: &[S]) -> io::Result<String> {
    let ut = kommando(socket).args(argumenter.iter().map(AsRef::as_ref)).output()?;
    if !ut.status.success() {
        return Err(io::Error::other(String::from_utf8_lossy(&ut.stderr).trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&ut.stdout).trim_end().to_string())
}

/// Kommandoen som én linje for control mode, med argumentene i anførselstegn.
pub fn linje<S: AsRef<str>>(argumenter: &[S]) -> String {
    let mut deler = argumenter.iter().map(AsRef::as_ref);
    let mut linje = deler.next().unwrap_or_default().to_string();
    for del in deler {
        linje.push(' ');
        linje.push_str(&sitat(del));
    }
    linje
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn linje_siterer_argumentene_men_ikke_kommandoen() {
        assert_eq!(linje(&["send-keys", "-t", "%1", "-l", "it's"]), r"send-keys '-t' '%1' '-l' 'it'\''s'");
    }
}
