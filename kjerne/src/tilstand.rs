//! Hva en Claude holder på med, lest av skjermbildet til panen.
//!
//! Oversatt fra `tilstand()` i sidebar-repoets `sesjoner.py`. Mønstrene er
//! skrevet for hånd framfor med regex, og testene under holder dem på linje
//! med originalen.

/// Claude Code setter terminaltittelen til en oppsummering med denne glyfen
/// foran, så Claude-paner kjennes igjen før noe skjermbilde er lest.
pub const TITTEL_GLYF: char = '✳';

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tilstand {
    Venter,
    Jobber,
    Klar,
    Shell,
}

impl Tilstand {
    /// Hvor mye tilstanden trenger brukeren. Høyest først.
    pub fn rang(self) -> u8 {
        match self {
            Tilstand::Venter => 3,
            Tilstand::Jobber => 2,
            Tilstand::Klar => 1,
            Tilstand::Shell => 0,
        }
    }

    pub fn navn(self) -> &'static str {
        match self {
            Tilstand::Venter => "venter",
            Tilstand::Jobber => "jobber",
            Tilstand::Klar => "klar",
            Tilstand::Shell => "shell",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lesning {
    pub tilstand: Tilstand,
    /// `venter`, en varighet som `4m`, eller tom.
    pub status: String,
    /// Kontekstforbruk som `31%`, eller tom.
    pub kontekst: String,
    /// Satt bare når Claude venter: spørsmålet over valglista.
    pub spørsmål: String,
}

/// `venter_ifølge_hook` er `@claude_state == "waiting"`, satt av
/// Notification-hooken. Den dekker bare permission-prompts, så en
/// AskUserQuestion-dialog ville sett ledig ut uten skjermlesingen.
pub fn les(skjerm: &str, venter_ifølge_hook: bool) -> Lesning {
    let linjer: Vec<&str> = skjerm.lines().map(str::trim_end).collect();
    let synlige: Vec<&str> = linjer.iter().copied().filter(|l| !l.trim().is_empty()).collect();
    let hale = &synlige[synlige.len().saturating_sub(8)..];
    let kontekst = kontekst(skjerm);

    let lesning = |tilstand, status: &str, spørsmål: String| Lesning {
        tilstand,
        status: status.to_string(),
        kontekst: kontekst.clone(),
        spørsmål,
    };

    for linje in hale.iter().rev() {
        if let Some(tid) = spinnertid(linje) {
            return lesning(Tilstand::Jobber, &tid, String::new());
        }
    }
    if hale.iter().any(|l| l.to_lowercase().contains("esc to interrupt")) {
        return lesning(Tilstand::Jobber, "", String::new());
    }

    // Claude sin egen statuslinje nederst betyr normalmodus. Den negative
    // testen trengs fordi @claude_state kan stå igjen på «waiting» etter at en
    // dialog er besvart, og fordi en besvart dialog ligger igjen i scrollbacken.
    let i_normalmodus = synlige[synlige.len().saturating_sub(3)..]
        .iter()
        .any(|l| er_statuslinje(l));
    if !i_normalmodus && (venter_ifølge_hook || hale.iter().any(|l| er_dialog(l))) {
        return lesning(Tilstand::Venter, "venter", spørsmål(&linjer));
    }

    lesning(Tilstand::Klar, "", String::new())
}

/// Bare footeren under en åpen dialog. Selve valgene («❯ 1. …») duger ikke som
/// markør: Claude skriver nummererte lister i vanlig svartekst også.
fn er_dialog(linje: &str) -> bool {
    ["Esc to cancel", "Enter to select", "Do you want to "]
        .iter()
        .any(|m| linje.contains(m))
}

fn er_statuslinje(linje: &str) -> bool {
    ["auto mode on", "til compact)", "accept edits on", "plan mode on"]
        .iter()
        .any(|m| linje.contains(m))
}

/// Første `<tall>%` fulgt av en parentes, som i `31% (…)`.
fn kontekst(skjerm: &str) -> String {
    for (i, _) in skjerm.match_indices('%') {
        let før = &skjerm[..i];
        let siffer = før.len() - før.trim_end_matches(|c: char| c.is_ascii_digit()).len();
        let etter = skjerm[i + 1..].trim_start();
        if siffer > 0 && etter.starts_with('(') {
            return format!("{}%", &før[før.len() - siffer..]);
        }
    }
    String::new()
}

/// Varigheten i spinnerlinja, f.eks. `2m` fra «✻ Finagling… (2m 56s · ↓ 10.3k
/// tokens)». Tiden kreves, ellers ville vanlig svartekst med ellipse og
/// parentes slått ut som jobbing.
fn spinnertid(linje: &str) -> Option<String> {
    for (i, _) in linje.match_indices('…') {
        // Ordet foran ellipsen må inneholde en stor bokstav et sted i løpet av
        // ord- og mellomromstegn rett foran, slik `[A-ZÆØÅ][\w æøå]*?…` krever.
        let ord = linje[..i]
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ' ');
        if !ord.into_iter().any(|c| c.is_ascii_uppercase() || "ÆØÅ".contains(c)) {
            continue;
        }
        let rest = linje[i + '…'.len_utf8()..].trim_start();
        let Some(rest) = rest.strip_prefix('(') else { continue };
        let mut rest = rest.trim_start();
        let tall = |enhet: char, rest: &mut &str| -> Option<String> {
            let siffer = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            if siffer == 0 || !rest[siffer..].starts_with(enhet) {
                return None;
            }
            let verdi = rest[..siffer].to_string();
            *rest = rest[siffer + 1..].trim_start();
            Some(verdi)
        };
        let timer = tall('h', &mut rest);
        let minutter = tall('m', &mut rest);
        let sekunder = tall('s', &mut rest);
        let tid = match (timer, minutter, sekunder) {
            (Some(t), m, _) => format!("{t}t{}m", m.as_deref().unwrap_or("0")),
            (None, Some(m), _) => format!("{m}m"),
            (None, None, Some(s)) => format!("{s}s"),
            (None, None, None) => continue,
        };
        return Some(tid);
    }
    None
}

/// Spørsmålet over valglista, slik at oversikten viser hva som blokkerer.
fn spørsmål(linjer: &[&str]) -> String {
    for linje in linjer[linjer.len().saturating_sub(40)..].iter().rev() {
        let tekst = linje.trim();
        if tekst.ends_with('?') && !tekst.starts_with(['❯', '│', '─']) {
            return tekst.to_string();
        }
    }
    String::new()
}

/// Oppgaven Claude har gitt seg selv i terminaltittelen, eller tom.
pub fn oppgave(tittel: &str, sti: &str) -> String {
    let tittel = tittel.trim_start_matches(TITTEL_GLYF).trim();
    let mappe = std::path::Path::new(sti)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    // «Claude Code» er tittelen før Claude har rukket å oppsummere noe.
    if tittel.is_empty() || tittel == "Claude Code" || tittel == mappe {
        return String::new();
    }
    tittel.to_string()
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn spinner_med_tid_er_jobbing() {
        let l = les("hei\n✻ Finagling… (2m 56s · ↓ 10.3k tokens)\n", false);
        assert_eq!(l.tilstand, Tilstand::Jobber);
        assert_eq!(l.status, "2m");
    }

    #[test]
    fn spinnertid_formateres_som_originalen() {
        assert_eq!(spinnertid("✻ Tenker… (1h 2m 3s)").as_deref(), Some("1t2m"));
        assert_eq!(spinnertid("✻ Tenker… (1h)").as_deref(), Some("1t0m"));
        assert_eq!(spinnertid("✻ Tenker… (56s · ↑ 2k)").as_deref(), Some("56s"));
        assert_eq!(spinnertid("✻ Tenker på Østen… (4m)").as_deref(), Some("4m"));
    }

    #[test]
    fn ellipse_uten_tid_er_ikke_jobbing() {
        assert_eq!(spinnertid("Se på dette… (som nevnt)"), None);
        assert_eq!(spinnertid("bare små bokstaver… (2m)"), None);
    }

    #[test]
    fn esc_to_interrupt_er_jobbing_uten_tid() {
        let l = les("noe\n  esc to interrupt\n", false);
        assert_eq!((l.tilstand, l.status.as_str()), (Tilstand::Jobber, ""));
    }

    #[test]
    fn åpen_dialog_venter_med_spørsmålet() {
        let skjerm = "Do you want to proceed?\n❯ 1. Yes\n  2. No\n\nEsc to cancel\n";
        let l = les(skjerm, false);
        assert_eq!(l.tilstand, Tilstand::Venter);
        assert_eq!(l.spørsmål, "Do you want to proceed?");
    }

    #[test]
    fn statuslinja_overstyrer_hooken() {
        let skjerm = "svar\n> \n  ⏵⏵ auto mode on · 12% (til compact)\n";
        let l = les(skjerm, true);
        assert_eq!(l.tilstand, Tilstand::Klar);
        assert_eq!(l.kontekst, "12%");
    }

    #[test]
    fn hooken_alene_gir_venter() {
        assert_eq!(les("> \n", true).tilstand, Tilstand::Venter);
    }

    #[test]
    fn spørsmål_hopper_over_valglinjer() {
        let linjer = ["Hva vil du?", "❯ 1. Ja?", "│ boks?"];
        assert_eq!(spørsmål(&linjer), "Hva vil du?");
    }

    #[test]
    fn oppgave_fra_tittel() {
        assert_eq!(oppgave("✳ Fiks testene", "/a/b"), "Fiks testene");
        assert_eq!(oppgave("✳ Claude Code", "/a/b"), "");
        assert_eq!(oppgave("✳ b", "/a/b"), "");
    }
}
