//! Kopimodus: en markør i panen med tastene fra copy-mode-vi i tmux, der `v`
//! markerer og `y` kopierer.

use nerdr_kjerne::sgr::Linje;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::markering::{self, Markering, Punkt};

/// Det kopimodus ser av panen. Bare linjene i bildet er kjent, så ord og
/// linjeslutt utenfor bildet finnes ikke.
pub struct Innhold {
    /// Rutene i hver linje i bildet, øverst først.
    ruter: Vec<Vec<String>>,
    historie: u32,
    rull: u32,
    bredde: u16,
    høyde: u16,
}

#[derive(Clone, Copy, PartialEq)]
enum Klasse {
    Mellomrom,
    Ord,
    Tegn,
}

impl Innhold {
    pub fn ny(bilde: &[Linje], historie: u32, rull: u32, bredde: u16, høyde: u16) -> Innhold {
        let ruter = bilde.iter().take(høyde as usize).map(|rad| markering::ruter(rad, bredde)).collect();
        Innhold { ruter, historie, rull, bredde, høyde }
    }

    fn topp(&self) -> i64 {
        self.historie as i64 - self.rull as i64
    }

    /// Nederste linje i panen.
    fn bunn(&self) -> i64 {
        self.historie as i64 + self.høyde as i64 - 1
    }

    fn linje(&self, linje: i64) -> Option<&[String]> {
        self.ruter.get(usize::try_from(linje - self.topp()).ok()?).map(Vec::as_slice)
    }

    /// Siste kolonne med noe annet enn mellomrom.
    fn siste(&self, linje: i64) -> Option<u16> {
        self.linje(linje)?.iter().rposition(|r| !r.trim().is_empty()).map(|x| x as u16)
    }

    fn første(&self, linje: i64) -> u16 {
        self.linje(linje).and_then(|rad| rad.iter().position(|r| !r.trim().is_empty())).unwrap_or(0) as u16
    }

    /// `store` er `W`, `B` og `E` i vi, der alt som ikke er mellomrom, er ord.
    fn klasse(&self, (linje, x): Punkt, store: bool) -> Klasse {
        let tegn = self.linje(linje).and_then(|rad| rad.get(x as usize)).and_then(|r| r.chars().next());
        match tegn {
            None => Klasse::Mellomrom,
            Some(c) if c.is_whitespace() => Klasse::Mellomrom,
            Some(_) if store => Klasse::Ord,
            Some(c) if c.is_alphanumeric() || c == '_' => Klasse::Ord,
            Some(_) => Klasse::Tegn,
        }
    }

    /// Ruta etter, over på neste linje når den er kjent.
    fn neste(&self, (linje, x): Punkt) -> Option<Punkt> {
        let rad = self.linje(linje)?;
        let x = (x as usize + 1..rad.len()).find(|&x| !rad[x].is_empty());
        match x {
            Some(x) => Some((linje, x as u16)),
            None => self.linje(linje + 1).map(|_| (linje + 1, 0)),
        }
    }

    fn forrige(&self, (linje, x): Punkt) -> Option<Punkt> {
        if x > 0 {
            return Some((linje, start(self.linje(linje)?, x - 1)));
        }
        let rad = self.linje(linje - 1)?;
        let siste = u16::try_from(rad.len()).ok()?.checked_sub(1)?;
        Some((linje - 1, start(rad, siste)))
    }

    fn ord_fram(&self, fra: Punkt, store: bool) -> Punkt {
        let klasse = self.klasse(fra, store);
        let mut p = fra;
        // Resten av ordet markøren står i. Et nytt ord starter på en ny linje.
        if klasse != Klasse::Mellomrom {
            loop {
                let Some(neste) = self.neste(p) else { return p };
                let ny_linje = neste.0 != p.0;
                p = neste;
                if ny_linje || self.klasse(p, store) != klasse {
                    break;
                }
            }
        }
        while self.klasse(p, store) == Klasse::Mellomrom {
            let Some(neste) = self.neste(p) else { return p };
            p = neste;
        }
        p
    }

    fn ord_slutt(&self, fra: Punkt, store: bool) -> Punkt {
        let Some(mut p) = self.neste(fra) else { return fra };
        while self.klasse(p, store) == Klasse::Mellomrom {
            let Some(neste) = self.neste(p) else { return p };
            p = neste;
        }
        let klasse = self.klasse(p, store);
        while let Some(neste) = self.neste(p).filter(|n| n.0 == p.0 && self.klasse(*n, store) == klasse) {
            p = neste;
        }
        p
    }

    fn ord_bak(&self, fra: Punkt, store: bool) -> Punkt {
        let Some(mut p) = self.forrige(fra) else { return fra };
        while self.klasse(p, store) == Klasse::Mellomrom {
            let Some(forrige) = self.forrige(p) else { return p };
            p = forrige;
        }
        let klasse = self.klasse(p, store);
        while let Some(forrige) = self.forrige(p).filter(|f| f.0 == p.0 && self.klasse(*f, store) == klasse) {
            p = forrige;
        }
        p
    }

    /// Rullingen som viser `linje`, med minst mulig endring fra `rull`.
    fn rull_for(&self, linje: i64, rull: i64) -> u32 {
        let (historie, høyde) = (self.historie as i64, self.høyde.max(1) as i64);
        let rull = rull.clamp(0, historie);
        let topp = historie - rull;
        let rull = match () {
            _ if linje < topp => historie - linje,
            _ if linje >= topp + høyde => historie - (linje - høyde + 1),
            _ => rull,
        };
        rull.clamp(0, historie) as u32
    }
}

/// Kolonnen der tegnet i `x` starter, når det er bredt.
fn start(rad: &[String], x: u16) -> u16 {
    let mut x = x.min(rad.len().saturating_sub(1) as u16);
    while x > 0 && rad[x as usize].is_empty() {
        x -= 1;
    }
    x
}

pub struct Kopimodus {
    pub pane: String,
    pub markør: Punkt,
    pub markering: Option<Markering>,
    /// Kolonnen `j` og `k` går til når linja er lang nok, som i vi. Etter `$`
    /// er den slutten av linja.
    kolonne: u16,
    /// `g` er trykket, og en `g` til går til toppen.
    g: bool,
}

enum Flytt {
    /// Kolonnen `j` og `k` går til, blir den nye.
    Til(Punkt),
    /// Linjer ned, og hvor mye visningen ruller opp, med kolonnen fra før.
    Linjer(i64, i64),
    /// Slutten av linja, også på linjene `j` og `k` går til etterpå.
    Slutt,
}

pub enum Utfall {
    /// Visningen skal ha denne rullingen, så markøren synes.
    Rull(u32),
    Kopier(Markering),
    Ut,
    /// Tasten betyr ingenting i kopimodus.
    Ingenting,
}

impl Kopimodus {
    /// Markøren starter der panen har sin, eller på siste linje med tekst.
    pub fn ny(pane: String, innhold: &Innhold, markør: Option<(u16, u16)>) -> Kopimodus {
        let markør = match markør {
            Some((x, y)) => (innhold.historie as i64 + y as i64, x),
            None => {
                let linje = (innhold.topp()..=innhold.bunn()).rev().find(|&l| innhold.siste(l).is_some());
                (linje.unwrap_or(innhold.bunn()), 0)
            }
        };
        Kopimodus { pane, markør, markering: None, kolonne: markør.1, g: false }
    }

    pub fn tast(&mut self, k: KeyEvent, innhold: &Innhold) -> Utfall {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let g = std::mem::take(&mut self.g);
        let (linje, topp) = (self.markør.0, innhold.topp());
        let halv = (innhold.høyde / 2).max(1) as i64;
        let side = innhold.høyde.max(1) as i64;
        let første = |linje: i64| Flytt::Til((linje, innhold.første(linje)));
        let flytt = match k.code {
            KeyCode::Char('c') if ctrl => return Utfall::Ut,
            KeyCode::Char('q') => return Utfall::Ut,
            KeyCode::Esc => {
                return match self.markering.take() {
                    Some(_) => Utfall::Rull(innhold.rull),
                    None => Utfall::Ut,
                };
            }
            KeyCode::Char('y') | KeyCode::Enter => return self.markering.take().map_or(Utfall::Ut, Utfall::Kopier),
            KeyCode::Char(c @ ('v' | 'V' | ' ')) => {
                self.marker(c == 'V');
                return Utfall::Rull(innhold.rull);
            }
            KeyCode::Char('o') => match &mut self.markering {
                Some(markering) => Flytt::Til(markering.snu()),
                None => return Utfall::Ingenting,
            },
            KeyCode::Char('g') if !g => {
                self.g = true;
                return Utfall::Rull(innhold.rull);
            }
            KeyCode::Char('g') => Flytt::Til((0, 0)),
            KeyCode::Char('u') if ctrl => Flytt::Linjer(-halv, halv),
            KeyCode::Char('d') if ctrl => Flytt::Linjer(halv, -halv),
            KeyCode::Char('b') if ctrl => Flytt::Linjer(-side, side),
            KeyCode::Char('f') if ctrl => Flytt::Linjer(side, -side),
            KeyCode::PageUp => Flytt::Linjer(-side, side),
            KeyCode::PageDown => Flytt::Linjer(side, -side),
            KeyCode::Char('k') | KeyCode::Up => Flytt::Linjer(-1, 0),
            KeyCode::Char('j') | KeyCode::Down => Flytt::Linjer(1, 0),
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => {
                Flytt::Til(innhold.forrige(self.markør).filter(|p| p.0 == linje).unwrap_or(self.markør))
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let siste = innhold.siste(linje).unwrap_or(0);
                Flytt::Til(innhold.neste(self.markør).filter(|p| p.0 == linje && p.1 <= siste).unwrap_or(self.markør))
            }
            KeyCode::Char(c @ ('w' | 'W')) => Flytt::Til(innhold.ord_fram(self.markør, c == 'W')),
            KeyCode::Char(c @ ('e' | 'E')) => Flytt::Til(innhold.ord_slutt(self.markør, c == 'E')),
            KeyCode::Char(c @ ('b' | 'B')) => Flytt::Til(innhold.ord_bak(self.markør, c == 'B')),
            KeyCode::Char('0') | KeyCode::Home => Flytt::Til((linje, 0)),
            KeyCode::Char('^') => første(linje),
            KeyCode::Char('$') | KeyCode::End => Flytt::Slutt,
            KeyCode::Char('G') => første(innhold.bunn()),
            KeyCode::Char('H') => første(topp),
            KeyCode::Char('M') => første(topp + side / 2),
            KeyCode::Char('L') => første(topp + side - 1),
            _ => return Utfall::Ingenting,
        };
        let mut rull = innhold.rull as i64;
        (self.markør, self.kolonne) = match flytt {
            Flytt::Til(punkt) => (punkt, punkt.1),
            Flytt::Linjer(steg, rull_steg) => {
                rull += rull_steg;
                let linje = (linje + steg).clamp(0, innhold.bunn());
                let x = match innhold.linje(linje) {
                    Some(rad) => start(rad, self.kolonne.min(innhold.siste(linje).unwrap_or(0))),
                    None => self.kolonne.min(innhold.bredde.saturating_sub(1)),
                };
                ((linje, x), self.kolonne)
            }
            Flytt::Slutt => ((linje, innhold.siste(linje).unwrap_or(0)), u16::MAX),
        };
        if let Some(markering) = &mut self.markering {
            markering.dra(self.markør);
        }
        Utfall::Rull(innhold.rull_for(self.markør.0, rull))
    }

    /// `v` og `V` slår av markeringen de lager, og bytter til den andre.
    fn marker(&mut self, hele_linjer: bool) {
        self.markering = match self.markering.take() {
            Some(m) if m.hele_linjer == hele_linjer => None,
            Some(mut m) => {
                m.hele_linjer = hele_linjer;
                Some(m)
            }
            None => Some(Markering::fra_tastene(self.pane.clone(), self.markør, hele_linjer)),
        };
    }
}

#[cfg(test)]
mod tester {
    use super::*;
    use nerdr_kjerne::sgr::tolk;

    fn bilde(linjer: &[&str]) -> Vec<Linje> {
        tolk(&linjer.iter().map(|l| l.to_string()).collect::<Vec<_>>())
    }

    /// Linjene står i en pane uten historikk, så linje 0 er den første.
    fn innhold(linjer: &[&str]) -> Innhold {
        Innhold::ny(&bilde(linjer), 0, 0, 20, linjer.len() as u16)
    }

    fn trykk(modus: &mut Kopimodus, innhold: &Innhold, taster: &str) -> Vec<Utfall> {
        taster.chars().map(|c| modus.tast(KeyEvent::from(KeyCode::Char(c)), innhold)).collect()
    }

    fn kopiert(linjer: &[&str], markør: (u16, u16), taster: &str) -> String {
        let i = innhold(linjer);
        let mut modus = Kopimodus::ny("%1".into(), &i, Some(markør));
        trykk(&mut modus, &i, taster);
        match modus.tast(KeyEvent::from(KeyCode::Char('y')), &i) {
            Utfall::Kopier(m) => m.tekst(&bilde(linjer), 0, 20),
            _ => panic!("ingen markering"),
        }
    }

    #[test]
    fn starter_på_siste_linje_med_tekst_uten_markør_fra_panen() {
        let modus = Kopimodus::ny("%1".into(), &innhold(&["en", "to", "", ""]), None);
        assert_eq!(modus.markør, (1, 0));
    }

    #[test]
    fn starter_der_markøren_til_panen_står_også_når_den_er_rullet() {
        let i = Innhold::ny(&[], 30, 5, 20, 10);
        assert_eq!(Kopimodus::ny("%1".into(), &i, Some((4, 2))).markør, (32, 4));
    }

    #[test]
    fn ord_fram_bak_og_til_slutten() {
        let i = innhold(&["hei, verden", "neste"]);
        let mut m = Kopimodus::ny("%1".into(), &i, Some((0, 0)));
        let mut etter = |taster| {
            trykk(&mut m, &i, taster);
            m.markør
        };
        assert_eq!(etter("w"), (0, 3));
        assert_eq!(etter("w"), (0, 5));
        assert_eq!(etter("e"), (0, 10));
        assert_eq!(etter("w"), (1, 0));
        assert_eq!(etter("b"), (0, 5));
        assert_eq!(etter("b"), (0, 3));
        assert_eq!(etter("0W"), (0, 5));
    }

    #[test]
    fn j_og_k_holder_kolonnen_over_kortere_linjer() {
        let i = innhold(&["en lang linje", "kort", "enda en lang linje"]);
        let mut m = Kopimodus::ny("%1".into(), &i, Some((10, 0)));
        let mut etter = |taster| {
            trykk(&mut m, &i, taster);
            m.markør
        };
        assert_eq!(etter("j"), (1, 3));
        assert_eq!(etter("j"), (2, 10));
        assert_eq!(etter("$k"), (1, 3));
        assert_eq!(etter("j"), (2, 17));
    }

    #[test]
    fn l_stopper_ved_siste_tegn_og_h_ved_første() {
        let i = innhold(&["abc   "]);
        let mut m = Kopimodus::ny("%1".into(), &i, Some((0, 0)));
        trykk(&mut m, &i, "llll");
        assert_eq!(m.markør, (0, 2));
        trykk(&mut m, &i, "hhhh");
        assert_eq!(m.markør, (0, 0));
    }

    #[test]
    fn brede_tegn_tar_ett_steg() {
        let i = innhold(&["中文 ok"]);
        let mut m = Kopimodus::ny("%1".into(), &i, Some((0, 0)));
        trykk(&mut m, &i, "l");
        assert_eq!(m.markør, (0, 2));
        trykk(&mut m, &i, "w");
        assert_eq!(m.markør, (0, 5));
    }

    #[test]
    fn v_markerer_fra_markøren_og_y_kopierer() {
        assert_eq!(kopiert(&["hei, verden"], (5, 0), "ve"), "verden");
    }

    #[test]
    fn stor_v_kopierer_hele_linjer() {
        assert_eq!(kopiert(&["en", "to", "tre"], (1, 1), "Vj"), "to\ntre");
    }

    #[test]
    fn o_flytter_den_andre_enden() {
        assert_eq!(kopiert(&["hei, verden"], (5, 0), "veoB"), "hei, verden");
    }

    #[test]
    fn esc_fjerner_markeringen_før_den_går_ut() {
        let i = innhold(&["hei"]);
        let mut m = Kopimodus::ny("%1".into(), &i, Some((0, 0)));
        trykk(&mut m, &i, "v");
        assert!(matches!(m.tast(KeyEvent::from(KeyCode::Esc), &i), Utfall::Rull(0)));
        assert!(m.markering.is_none());
        assert!(matches!(m.tast(KeyEvent::from(KeyCode::Esc), &i), Utfall::Ut));
    }

    #[test]
    fn markøren_over_bildet_ruller_opp_og_gg_går_til_toppen() {
        // Ti linjer historikk og fem på skjermen, rullet helt ned.
        let i = Innhold::ny(&[], 10, 0, 20, 5);
        let mut m = Kopimodus::ny("%1".into(), &i, Some((0, 0)));
        assert!(matches!(trykk(&mut m, &i, "k")[..], [Utfall::Rull(1)]));
        assert!(matches!(trykk(&mut m, &i, "gg")[..], [_, Utfall::Rull(10)]));
        assert_eq!(m.markør, (0, 0));
    }

    #[test]
    fn halv_side_ruller_like_mye_som_markøren_flytter() {
        let i = Innhold::ny(&[], 10, 0, 20, 6);
        let mut m = Kopimodus::ny("%1".into(), &i, Some((0, 4)));
        let opp = m.tast(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL), &i);
        assert!(matches!(opp, Utfall::Rull(3)));
        assert_eq!(m.markør, (11, 0));
    }
}
