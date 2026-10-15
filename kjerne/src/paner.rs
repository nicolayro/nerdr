//! Panene på tmux-serveren, lest fra `list-panes -a`.

/// Formatet til `list-panes -a -F`. Rekkefølgen må stemme med `Pane::fra_linje`.
pub const FELT: &str = "#{session_id}\t#{session_name}\t#{window_id}\t#{window_index}\t\
#{pane_id}\t#{pane_title}\t\
#{window_width}\t#{window_height}\t#{pane_width}\t#{pane_height}";

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
}

impl Pane {
    pub fn fra_linje(linje: &str) -> Option<Pane> {
        let f: Vec<&str> = linje.split('\t').collect();
        let [sesjon_id, sesjon, vindu_id, vindu, id, tittel, vb, vh, b, h] = f.as_slice() else {
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
        })
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn leser_list_panes_linje() {
        let p = Pane::fra_linje("$0\tmain\t@0\t0\t%0\t✳ Fiks\t176\t38\t140\t38").unwrap();
        assert_eq!((p.id.as_str(), p.bredde, p.vindu_bredde), ("%0", 140, 176));
        assert_eq!(Pane::fra_linje("for\tfå"), None);
    }
}
