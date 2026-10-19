//! Panene på tmux-serveren, lest fra `list-panes -a`.

use std::path::PathBuf;

/// Formatet til `list-panes -a -F`. Rekkefølgen må stemme med `Pane::fra_linje`.
pub const FELT: &str = "#{session_id}\t#{session_name}\t#{window_id}\t#{window_index}\t\
#{pane_id}\t#{pane_title}\t\
#{window_width}\t#{window_height}\t#{pane_width}\t#{pane_height}\t\
#{pane_left}\t#{pane_top}\t#{pane_active}\t#{window_visible_layout}\t#{window_active}\t#{window_name}\t\
#{@nerdr-sett}\t#{pane_current_path}";

/// Pane-optionen der nerdr skriver når du sist var inne i panen. Den lever like
/// lenge som panen, så nerdr trenger ingen fil for å huske den. Må stemme med
/// `FELT`.
pub const SETT: &str = "@nerdr-sett";

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
    /// Plassen i vinduet, fra øverst til venstre.
    pub venstre: u16,
    pub topp: u16,
    /// Panen tastene går til i vinduet.
    pub aktiv: bool,
    /// Oppsettet til vinduet, til `oppsett::tolk`. Paner som er skjult av
    /// zoom, er ikke med.
    pub oppsett: String,
    /// Vinduet er det gjeldende i sesjonen.
    pub vindu_aktiv: bool,
    pub vindunavn: String,
    /// Da du sist var inne i panen i nerdr, i millisekunder siden 1970.
    pub sett: Option<u64>,
    pub mappe: PathBuf,
}

impl Pane {
    pub fn fra_linje(linje: &str) -> Option<Pane> {
        // Mappa står sist, så en tab i den blir med.
        let f: Vec<&str> = linje.splitn(18, '\t').collect();
        let [sesjon_id, sesjon, vindu_id, vindu, id, tittel, vb, vh, b, h, x, y, aktiv, oppsett, vindu_aktiv, vindunavn, sett, mappe] =
            f.as_slice()
        else {
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
            venstre: x.parse().ok()?,
            topp: y.parse().ok()?,
            aktiv: *aktiv == "1",
            oppsett: oppsett.to_string(),
            vindu_aktiv: *vindu_aktiv == "1",
            vindunavn: vindunavn.to_string(),
            sett: sett.parse().ok(),
            mappe: PathBuf::from(mappe),
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

    #[test]
    fn leser_list_panes_linje() {
        let p = Pane::fra_linje("$0\tmain\t@0\t0\t%0\t✳ Fiks\t176\t38\t140\t38\t36\t0\t1\tb25d,176x38,0,0,0\t1\tzsh\t17\t/h/git/med\ttab").unwrap();
        assert_eq!((p.id.as_str(), p.bredde, p.vindu_bredde), ("%0", 140, 176));
        assert_eq!((p.venstre, p.aktiv, p.oppsett.as_str()), (36, true, "b25d,176x38,0,0,0"));
        assert_eq!((p.vindu_aktiv, p.vindunavn.as_str(), p.sett), (true, "zsh", Some(17)));
        assert_eq!(p.mappe, PathBuf::from("/h/git/med\ttab"));
        assert_eq!(Pane::fra_linje("for\tfå"), None);
    }

    #[test]
    fn oppgave_er_tittelen_etter_tegnet_til_claude() {
        let med = |tittel: &str| Pane { tittel: tittel.into(), ..Pane::fra_linje("$0\ts\t@0\t0\t%0\t\t1\t1\t1\t1\t0\t0\t1\t\t1\t\t\t").unwrap() };
        assert_eq!(med("✳ Fiks testene").oppgave(), Some("Fiks testene"));
        assert_eq!(med("⠐ Fiks testene").oppgave(), Some("Fiks testene"));
        assert_eq!(med("min-maskin.local").oppgave(), None);
        assert_eq!(med("claude-mods").oppgave(), None);
    }
}
