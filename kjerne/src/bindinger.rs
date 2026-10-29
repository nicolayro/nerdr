//! Bindingene i prefikstabellen til tmux, fra `list-keys -T prefix`, som
//! skriver hver av dem slik de ville stått i `.tmux.conf`:
//!
//! ```text
//! bind-key -r -T prefix H       resize-pane -L 5
//! bind-key    -T prefix \%      split-window -h
//! ```

#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    /// Navnet tmux gir tasten, som `%`, `C-o` eller `M-Up`.
    pub tast: String,
    /// `-r`: tasten kan trykkes igjen uten prefikset, innenfor `repeat-time`.
    pub gjentas: bool,
    pub kommando: String,
}

pub fn les(linjer: &[String]) -> Vec<Binding> {
    linjer.iter().filter_map(|l| binding(l)).collect()
}

fn binding(linje: &str) -> Option<Binding> {
    let mut rest = linje.trim_start().strip_prefix("bind-key")?;
    let mut gjentas = false;
    loop {
        let (ord, etter) = neste(rest)?;
        rest = etter;
        match ord.as_str() {
            "-r" => gjentas = true,
            "-T" | "-N" => rest = neste(rest)?.1,
            _ => {
                return Some(Binding { tast: ord, gjentas, kommando: rest.trim().to_string() });
            }
        }
    }
}

/// Kommandoene i en binding, delt på `\;`, med ordene uten anførselstegn.
pub fn kommandoer(kommando: &str) -> Vec<Vec<String>> {
    let mut ut = vec![Vec::new()];
    let mut rest = kommando;
    while let Some((ord, etter)) = neste(rest) {
        rest = etter;
        if ord == ";" {
            ut.push(Vec::new());
        } else if let Some(siste) = ut.last_mut() {
            siste.push(ord);
        }
    }
    ut.retain(|k| !k.is_empty());
    ut
}

/// Neste ord og det som står etter det. Anførselstegn og `\` fjernes, som
/// tmux gjør når den leser oppsettet.
fn neste(tekst: &str) -> Option<(String, &str)> {
    let tekst = tekst.trim_start();
    let mut ord = String::new();
    let mut tegn = tekst.char_indices();
    let mut sitat = None;
    while let Some((i, c)) = tegn.next() {
        match (sitat, c) {
            (None, c) if c.is_whitespace() => return Some((ord, &tekst[i..])),
            (None, '"' | '\'') => sitat = Some(c),
            (Some(s), c) if c == s => sitat = None,
            (Some('\''), c) => ord.push(c),
            (_, '\\') => ord.extend(tegn.next().map(|(_, c)| c)),
            (_, c) => ord.push(c),
        }
    }
    (!ord.is_empty()).then_some((ord, ""))
}

#[cfg(test)]
mod tester {
    use super::*;

    fn linjer(tekst: &str) -> Vec<String> {
        tekst.lines().map(String::from).collect()
    }

    #[test]
    fn tasten_flaggene_og_kommandoen() {
        let b = les(&linjer(
            "bind-key -r -T prefix H       resize-pane -L 5\n\
             bind-key    -T prefix \\%      split-window -h\n\
             bind-key    -T prefix \\;      last-pane\n\
             bind-key    -T root   M-h     select-pane -L",
        ));
        assert_eq!(b[0], Binding { tast: "H".into(), gjentas: true, kommando: "resize-pane -L 5".into() });
        assert_eq!((b[1].tast.as_str(), b[1].gjentas, b[1].kommando.as_str()), ("%", false, "split-window -h"));
        assert_eq!(b[2].tast, ";");
        assert_eq!(b[3].tast, "M-h");
    }

    #[test]
    fn kommandoene_deles_på_semikolon() {
        let k = kommandoer(r#"source-file "~/a b.conf" \; display-message "lastet på nytt""#);
        assert_eq!(k, [vec!["source-file", "~/a b.conf"], vec!["display-message", "lastet på nytt"]]);
        assert_eq!(kommandoer(r#"confirm-before -p "kill-window #W? (y/n)" kill-window"#)[0].len(), 4);
    }
}
