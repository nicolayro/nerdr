//! Klient for tmux control mode (`tmux -C`).
//!
//! Én tmux-prosess holdes gående, og alle kommandoer går gjennom den. Ingen
//! nye prosesser per oppslag, og tmux melder selv fra når et pane skriver.
//!
//! Svarene kommer i samme rekkefølge som kommandoene ble sendt, så den som
//! sender, holder selv en kø over hva hvert svar gjelder.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Stdio};
use std::thread;

use crate::tmux;

#[derive(Debug, PartialEq)]
pub enum Melding {
    /// Svaret på én kommando. `ok` er usann når tmux svarte med `%error`.
    Svar { ok: bool, linjer: Vec<String> },
    /// Et pane i sesjonen klienten er koblet til, har skrevet noe. `data` er
    /// slik tmux sender det, med kontrolltegn som oktale escapes (`\033`).
    Output { pane: String, data: String },
    /// Sesjoner, vinduer eller layout er endret et sted på serveren.
    Endret,
    /// Tilkoblingen er borte, med grunnen tmux oppga.
    Slutt(String),
}

#[derive(Default)]
pub struct Tolker {
    blokk: Option<Blokk>,
    grunn: String,
}

struct Blokk {
    /// `%end`- eller `%error`-linja som avslutter blokka, uten prefiks. Den
    /// gjentar tid, nummer og flagg fra `%begin`, så en linje i et svar som
    /// tilfeldigvis starter med `%end`, ikke avslutter for tidlig.
    hale: String,
    /// Blokka fra selve `attach` har flagg 0 og er ikke svar på noe vi sendte.
    egen: bool,
    linjer: Vec<String>,
}

impl Tolker {
    pub fn linje(&mut self, linje: &str) -> Option<Melding> {
        if let Some(blokk) = &mut self.blokk {
            let ferdig = match linje.split_once(' ') {
                Some(("%end", hale)) if hale == blokk.hale => Some(true),
                Some(("%error", hale)) if hale == blokk.hale => Some(false),
                _ => None,
            };
            let Some(ok) = ferdig else {
                blokk.linjer.push(linje.to_string());
                return None;
            };
            let blokk = self.blokk.take()?;
            return blokk.egen.then_some(Melding::Svar { ok, linjer: blokk.linjer });
        }

        let (navn, rest) = linje.split_once(' ').unwrap_or((linje, ""));
        match navn {
            "%begin" => {
                self.blokk = Some(Blokk {
                    egen: rest.rsplit(' ').next() != Some("0"),
                    hale: rest.to_string(),
                    linjer: Vec::new(),
                });
                None
            }
            "%output" => {
                let (pane, data) = rest.split_once(' ').unwrap_or((rest, ""));
                Some(Melding::Output { pane: pane.to_string(), data: data.to_string() })
            }
            "%exit" => {
                self.grunn = rest.to_string();
                None
            }
            // %session-changed gjelder denne klienten selv, og kommer av at vi
            // byttet sesjon. Den endrer ikke oversikten.
            "%sessions-changed" | "%session-renamed" | "%session-window-changed"
            | "%window-add" | "%window-close" | "%window-renamed" | "%window-pane-changed"
            | "%unlinked-window-add" | "%unlinked-window-close"
            | "%unlinked-window-renamed" | "%layout-change" => Some(Melding::Endret),
            _ => None,
        }
    }
}

pub struct Kontroll {
    inn: ChildStdin,
    barn: Child,
}

impl Kontroll {
    /// Kobler til sesjonen tmux selv velger (sist brukte), på serveren med
    /// navnet `socket` (som `tmux -L`) eller standardserveren. Har serveren
    /// ingen sesjoner, eller kjører den ikke, lages en. `melde` kalles fra en
    /// egen tråd for hver melding.
    pub fn start(
        socket: Option<&str>,
        mut melde: impl FnMut(Melding) + Send + 'static,
    ) -> io::Result<Kontroll> {
        // attach-session uten sesjoner svarer i blokka fra selve oppkoblingen,
        // som tolkeren hopper over, så feilen ville kommet uten grunn.
        let finnes = utenfra(socket).arg("has-session").stdout(Stdio::null()).stderr(Stdio::null()).status()?.success();
        let mut barn = utenfra(socket)
            .args(["-C", if finnes { "attach-session" } else { "new-session" }])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let inn = barn.stdin.take().expect("stdin er piped");
        let ut = barn.stdout.take().expect("stdout er piped");
        let mut feil = barn.stderr.take().expect("stderr er piped");

        thread::spawn(move || {
            let mut tolker = Tolker::default();
            let mut leser = BufReader::new(ut);
            let mut buf = Vec::new();
            while matches!(leser.read_until(b'\n', &mut buf), Ok(n) if n > 0) {
                let linje = String::from_utf8_lossy(&buf);
                if let Some(melding) = tolker.linje(linje.trim_end_matches('\n')) {
                    melde(melding);
                }
                buf.clear();
            }
            let mut grunn = std::mem::take(&mut tolker.grunn);
            if grunn.is_empty() {
                let _ = feil.read_to_string(&mut grunn);
            }
            melde(Melding::Slutt(grunn.trim().to_string()));
        });

        Ok(Kontroll { inn, barn })
    }

    pub fn send(&mut self, kommando: &str) -> io::Result<()> {
        self.inn.write_all(format!("{kommando}\n").as_bytes())
    }
}

/// Serveren `Kontroll::start` kobler til, med `-L socket` eller
/// standardserveren, uansett om nerdr selv står i en tmux.
fn utenfra(socket: Option<&str>) -> std::process::Command {
    let mut tmux = tmux::kommando(socket);
    // Uten dette nekter tmux å koble til fra innsiden av en annen tmux.
    tmux.env_remove("TMUX");
    tmux
}

/// Om nerdr står i en pane på serveren `Kontroll::start` ville koblet til.
/// Der tar tmux prefikset før nerdr ser det, og vinduet nerdr står i, kan
/// vises i nerdr og krympes til boksen sin, om og om igjen.
pub fn inne_i(socket: Option<&str>) -> bool {
    // `$TMUX` er `socket,pid,sesjon`, med pid-en til serveren.
    let Some(pid) = std::env::var("TMUX").ok().and_then(|t| Some(t.split(',').nth(1)?.to_string())) else {
        return false;
    };
    utenfra(socket)
        .args(["display-message", "-p", "#{pid}"])
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|ut| ut.status.success() && String::from_utf8_lossy(&ut.stdout).trim() == pid)
}

impl Drop for Kontroll {
    fn drop(&mut self) {
        let _ = self.barn.kill();
        let _ = self.barn.wait();
    }
}

/// Om panen er midt i en synkronisert oppdatering (DEC-modus 2026) etter
/// `data`, eller `None` når `data` verken slår den på eller av.
///
/// Claude Code tømmer og tegner hele skjermen i hver oppdatering, pakket i
/// denne modusen. En terminal viser ingenting før den slås av igjen, men
/// `capture-pane` leser rutenettet slik det står, også midt i.
pub fn synkronisert_etter(data: &str) -> Option<bool> {
    let på = data.rfind(r"\033[?2026h");
    let av = data.rfind(r"\033[?2026l");
    if på.is_none() && av.is_none() {
        return None;
    }
    Some(på > av)
}

/// Setter et argument i enkle anførselstegn for tmux sin kommandoparser.
pub fn sitat(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', r"'\''"))
}

/// Setter et argument i doble anførselstegn, for tekst som kan ha linjeskift.
/// I control mode er hver linje en kommando, og bare doble anførselstegn tar
/// escapes som `\n`. Der utvider tmux også `$` og `~`, så de escapes.
pub fn dobbelt_sitat(arg: &str) -> String {
    let mut ut = String::from('"');
    for c in arg.chars() {
        match c {
            '\\' | '"' | '$' | '~' => {
                ut.push('\\');
                ut.push(c);
            }
            '\n' => ut.push_str(r"\n"),
            '\r' => ut.push_str(r"\r"),
            '\t' => ut.push_str(r"\t"),
            c if c.is_ascii_control() => ut.push_str(&format!("\\{:03o}", c as u32)),
            c => ut.push(c),
        }
    }
    ut.push('"');
    ut
}

#[cfg(test)]
mod tester {
    use super::*;

    fn kjør(linjer: &[&str]) -> Vec<Melding> {
        let mut t = Tolker::default();
        linjer.iter().filter_map(|l| t.linje(l)).collect()
    }

    #[test]
    fn attach_blokka_er_ikke_et_svar() {
        let m = kjør(&[
            "%begin 1 440 0",
            "%end 1 440 0",
            "%session-changed $0 test",
            "%begin 1 443 1",
            "%3\tannen",
            "%end 1 443 1",
        ]);
        assert_eq!(m, vec![Melding::Svar { ok: true, linjer: vec!["%3\tannen".into()] }]);
    }

    #[test]
    fn feil_og_output() {
        let m = kjør(&[
            "%output %0 \\033[1mhei",
            "%begin 2 455 1",
            "parse error: unknown command: x",
            "%error 2 455 1",
            "%layout-change @1 abc",
        ]);
        assert_eq!(m[0], Melding::Output { pane: "%0".into(), data: r"\033[1mhei".into() });
        assert!(matches!(&m[1], Melding::Svar { ok: false, .. }));
        assert_eq!(m[2], Melding::Endret);
    }

    #[test]
    fn end_i_innholdet_avslutter_ikke_blokka() {
        let m = kjør(&["%begin 3 9 1", "%end 0 0 0", "%end 3 9 1"]);
        assert_eq!(m, vec![Melding::Svar { ok: true, linjer: vec!["%end 0 0 0".into()] }]);
    }

    #[test]
    fn siste_synkmarkør_avgjør() {
        assert_eq!(synkronisert_etter(r"\033[?2026h\033[H"), Some(true));
        assert_eq!(synkronisert_etter(r"tekst\033[?2026l"), Some(false));
        assert_eq!(synkronisert_etter(r"\033[?2026l\033[?2026h"), Some(true));
        assert_eq!(synkronisert_etter("bare tekst"), None);
    }

    #[test]
    fn sitat_tåler_apostrof() {
        assert_eq!(sitat("it's"), r"'it'\''s'");
    }

    #[test]
    fn dobbelt_sitat_escaper_det_tmux_utvider() {
        assert_eq!(dobbelt_sitat("a\tb\nc"), r#""a\tb\nc""#);
        assert_eq!(dobbelt_sitat(r#"~ $HOME "q" \"#), r#""\~ \$HOME \"q\" \\""#);
        assert_eq!(dobbelt_sitat("æ\x01\x7f"), r#""æ\001\177""#);
    }
}
