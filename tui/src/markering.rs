//! Tekst du markerer med musen i en pane som ikke har slått på mus selv.

use std::io::Write;
use std::ops::Range;
use std::process::{Command, Stdio};
use std::thread;

use nerdr_kjerne::sgr::Linje;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;

/// Linja regnet fra toppen av historikken, og kolonnen. Da står markeringen på
/// teksten når du ruller, og når ny tekst skyver den opp.
pub type Punkt = (i64, u16);

pub struct Markering {
    pub pane: String,
    fra: Punkt,
    til: Punkt,
    /// Knappen er fortsatt nede.
    pub drar: bool,
}

impl Markering {
    pub fn ny(pane: String, punkt: Punkt) -> Markering {
        Markering { pane, fra: punkt, til: punkt, drar: true }
    }

    pub fn dra(&mut self, punkt: Punkt) {
        self.til = punkt;
    }

    /// Et klikk uten at musen flyttet seg.
    pub fn tom(&self) -> bool {
        self.fra == self.til
    }

    fn ordnet(&self) -> (Punkt, Punkt) {
        if self.fra <= self.til { (self.fra, self.til) } else { (self.til, self.fra) }
    }

    /// Kolonnene som er markert på `linje`, i en pane som er `bredde` bred.
    pub fn kolonner(&self, linje: i64, bredde: u16) -> Option<Range<u16>> {
        let ((første, fra), (siste, til)) = self.ordnet();
        if linje < første || linje > siste {
            return None;
        }
        let fra = if linje == første { fra } else { 0 };
        let til = if linje == siste { til + 1 } else { bredde };
        let til = til.min(bredde);
        (fra < til).then_some(fra..til)
    }

    /// Det som er markert i `bilde`, der første linje er `topp`. Linjer som
    /// ikke er i bildet, er utelatt, og tomme linjer først og sist også, så
    /// det å dra inn i tomrommet under teksten ikke gir linjeskift.
    pub fn tekst(&self, bilde: &[Linje], topp: i64, bredde: u16) -> String {
        let ((første, _), (siste, _)) = self.ordnet();
        let linjer: Vec<String> = (første..=siste)
            .filter_map(|linje| {
                let rad = bilde.get(usize::try_from(linje - topp).ok()?)?;
                let kolonner = self.kolonner(linje, bredde)?;
                Some(celler(rad, bredde, kolonner).trim_end().to_string())
            })
            .collect();
        linjer.join("\n").trim_matches('\n').to_string()
    }
}

/// Kolonnene fram til siste tegn som ikke er mellomrom. Bare de vises som
/// markert, fordi det er bare de som kopieres.
pub fn innhold(rad: &Linje) -> u16 {
    let tekst: String = rad.iter().map(|bit| bit.tekst.as_str()).collect();
    Span::raw(tekst.trim_end()).width() as u16
}

/// Raden tegnes slik `tegn_bilde` gjør, så kolonnene er de samme som på
/// skjermen, også med brede tegn.
fn celler(rad: &Linje, bredde: u16, kolonner: Range<u16>) -> String {
    let mut buf = Buffer::empty(Rect::new(0, 0, bredde, 1));
    let mut x = 0;
    for bit in rad {
        if x >= bredde {
            break;
        }
        x = buf.set_stringn(x, 0, &bit.tekst, (bredde - x) as usize, Style::new()).0;
    }
    let bred = |x: u16| Span::raw(buf[(x, 0)].symbol()).width() > 1;
    kolonner.filter(|&x| x == 0 || !bred(x - 1)).map(|x| buf[(x, 0)].symbol()).collect()
}

/// Til utklippstavla med det første programmet som finnes. Gir `false` når
/// ingen av dem finnes.
pub fn kopier(tekst: &str) -> bool {
    let programmer: [(&str, &[&str]); 4] = [
        ("pbcopy", &[]),
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard"]),
        ("xsel", &["--clipboard", "--input"]),
    ];
    for (program, args) in programmer {
        let Ok(mut barn) =
            Command::new(program).args(args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
        else {
            continue;
        };
        if let Some(mut inn) = barn.stdin.take() {
            let _ = inn.write_all(tekst.as_bytes());
        }
        // xclip og wl-copy blir værende til noen andre kopierer.
        thread::spawn(move || barn.wait());
        return true;
    }
    false
}

#[cfg(test)]
mod tester {
    use super::*;
    use nerdr_kjerne::sgr::tolk;

    fn bilde(linjer: &[&str]) -> Vec<Linje> {
        tolk(&linjer.iter().map(|l| l.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn kolonnene_på_første_midterste_og_siste_linje() {
        let mut m = Markering::ny("%1".into(), (5, 3));
        m.dra((7, 2));
        assert_eq!(m.kolonner(4, 10), None);
        assert_eq!(m.kolonner(5, 10), Some(3..10));
        assert_eq!(m.kolonner(6, 10), Some(0..10));
        assert_eq!(m.kolonner(7, 10), Some(0..3));
        assert_eq!(m.kolonner(8, 10), None);
    }

    #[test]
    fn baklengs_er_det_samme_som_forlengs() {
        let mut m = Markering::ny("%1".into(), (7, 2));
        m.dra((5, 3));
        assert_eq!(m.kolonner(5, 10), Some(3..10));
        assert_eq!(m.kolonner(7, 10), Some(0..3));
        let mut m = Markering::ny("%1".into(), (5, 6));
        m.dra((5, 2));
        assert_eq!(m.kolonner(5, 10), Some(2..7));
    }

    #[test]
    fn teksten_uten_mellomrom_på_slutten() {
        let mut m = Markering::ny("%1".into(), (11, 2));
        m.dra((12, 4));
        let b = bilde(&["først", "en \x1b[1mfet\x1b[0m linje   ", "andre linje"]);
        assert_eq!(m.tekst(&b, 10, 20), " fet linje\nandre");
    }

    #[test]
    fn brede_tegn_kopieres_én_gang() {
        let mut m = Markering::ny("%1".into(), (0, 0));
        m.dra((0, 7));
        assert_eq!(m.tekst(&bilde(&["中文 rød"]), 0, 20), "中文 rød");
    }

    #[test]
    fn tomme_linjer_først_og_sist_utelates_men_ikke_innrykket() {
        let mut m = Markering::ny("%1".into(), (0, 0));
        m.dra((4, 9));
        let b = bilde(&["", "  innrykk", "", "neste", "     "]);
        assert_eq!(m.tekst(&b, 0, 10), "  innrykk\n\nneste");
    }

    #[test]
    fn innholdet_slutter_ved_siste_tegn() {
        assert_eq!(innhold(&bilde(&["~$ %      [main]   "])[0]), 16);
        assert_eq!(innhold(&bilde(&["中文  "])[0]), 4);
        assert_eq!(innhold(&bilde(&["    "])[0]), 0);
    }

    #[test]
    fn linjer_utenfor_bildet_utelates() {
        let mut m = Markering::ny("%1".into(), (-1, 0));
        m.dra((0, 2));
        assert_eq!(m.tekst(&bilde(&["abc"]), 0, 10), "abc");
    }
}
