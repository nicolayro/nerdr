//! Panene på tmux-serveren, lest fra `list-panes -a`.

/// Formatet til `list-panes -a -F`. Rekkefølgen må stemme med `Pane::fra_linje`.
pub const FELT: &str = "#{session_id}\t#{session_name}\t#{window_id}\t#{window_index}\t\
#{pane_id}\t#{pane_title}\t\
#{window_width}\t#{window_height}\t#{pane_width}\t#{pane_height}\t\
#{pane_current_command}\t#{@nerdr-claude}";

#[derive(Clone, Debug, PartialEq)]
pub struct Pane {
    pub sesjon_id: String,
    pub sesjon: String,
    pub vindu_id: String,
    pub vindu: String,
    pub id: String,
    pub tittel: String,
    pub vindu_bredde: u16,
    pub vindu_høyde: u16,
    pub bredde: u16,
    pub høyde: u16,
    /// Programmet som kjører i forgrunnen, som `zsh` eller `vim`.
    pub kommando: String,
    /// Claude-panen en terminal hører til, fra `terminal::OPSJON`.
    pub claude: Option<String>,
}

impl Pane {
    pub fn fra_linje(linje: &str) -> Option<Pane> {
        let f: Vec<&str> = linje.split('\t').collect();
        let [sesjon_id, sesjon, vindu_id, vindu, id, tittel, vb, vh, b, h, kommando, claude] = f.as_slice() else {
            return None;
        };
        Some(Pane {
            sesjon_id: sesjon_id.to_string(),
            sesjon: sesjon.to_string(),
            vindu_id: vindu_id.to_string(),
            vindu: vindu.to_string(),
            id: id.to_string(),
            tittel: tittel.to_string(),
            vindu_bredde: vb.parse().ok()?,
            vindu_høyde: vh.parse().ok()?,
            bredde: b.parse().ok()?,
            høyde: h.parse().ok()?,
            kommando: kommando.to_string(),
            claude: (!claude.is_empty()).then(|| claude.to_string()),
        })
    }

    /// Oppgaven Claude Code skriver i tittelen, etter tegnet som viser om den
    /// jobber (`✳ Fiks testene`). Uten et slikt tegn er tittelen satt av noe
    /// annet, som vertsnavnet.
    pub fn oppgave(&self) -> Option<&str> {
        let mut tegn = self.tittel.chars();
        let første = tegn.next()?;
        let resten = tegn.as_str().strip_prefix(' ')?.trim();
        (!første.is_alphanumeric() && !resten.is_empty()).then_some(resten)
    }
}

#[cfg(test)]
mod tester {
    use super::*;
    use crate::terminal;

    #[test]
    fn leser_list_panes_linje() {
        let p = Pane::fra_linje("$0\tmain\t@0\t0\t%0\t✳ Fiks\t176\t38\t140\t38\tzsh\t").unwrap();
        assert_eq!((p.id.as_str(), p.bredde, p.vindu_bredde), ("%0", 140, 176));
        assert_eq!((p.kommando.as_str(), p.claude), ("zsh", None));
        let t = Pane::fra_linje("$0\tmain\t@1\t1\t%4\t\t176\t38\t176\t38\tzsh\t%0").unwrap();
        assert_eq!(t.claude.as_deref(), Some("%0"));
        assert_eq!(Pane::fra_linje("for\tfå"), None);
    }

    #[test]
    fn felt_leser_opsjonen_terminalen_merkes_med() {
        assert!(FELT.ends_with(&format!("#{{{}}}", terminal::OPSJON)));
    }

    #[test]
    fn oppgave_er_tittelen_etter_tegnet_til_claude() {
        let med = |tittel: &str| Pane { tittel: tittel.into(), ..Pane::fra_linje("$0\ts\t@0\t0\t%0\t\t1\t1\t1\t1\t\t").unwrap() };
        assert_eq!(med("✳ Fiks testene").oppgave(), Some("Fiks testene"));
        assert_eq!(med("⠐ Fiks testene").oppgave(), Some("Fiks testene"));
        assert_eq!(med("min-maskin.local").oppgave(), None);
        assert_eq!(med("claude-mods").oppgave(), None);
    }
}
