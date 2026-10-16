//! Oppsettet til et vindu, fra `#{window_visible_layout}`: et tre der hver
//! celle er en pane, eller celler side om side eller over hverandre.
//!
//! tmux 3.8 skriver det som JSON, med `t` lik `p` for en pane, `h` for side
//! om side og `v` for over hverandre. Eldre versjoner skriver
//! `sjekksum,BxH,X,Y` fulgt av `,<pane>` for en pane, `{…}` for side om side
//! eller `[…]` for over hverandre, med cellene kommaseparert.

use serde::Deserialize;

#[derive(Clone, Debug, PartialEq)]
pub struct Celle {
    pub bredde: u16,
    pub høyde: u16,
    pub innhold: Innhold,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Innhold {
    /// Pane-id-en, som `%3`.
    Pane(String),
    SideOmSide(Vec<Celle>),
    OverHverandre(Vec<Celle>),
}

pub fn tolk(tekst: &str) -> Option<Celle> {
    if tekst.starts_with('{') {
        return fra_json(&serde_json::from_str::<Json>(tekst).ok()?.l);
    }
    let (_, rest) = tekst.split_once(',')?;
    let mut tegn = rest.chars().peekable();
    let celle = celle(&mut tegn)?;
    tegn.next().is_none().then_some(celle)
}

#[derive(Deserialize)]
struct Json {
    #[serde(rename = "L")]
    l: JsonCelle,
}

#[derive(Deserialize)]
struct JsonCelle {
    t: String,
    w: u16,
    h: u16,
    #[serde(rename = "I")]
    id: Option<String>,
    #[serde(default)]
    c: Vec<JsonCelle>,
}

fn fra_json(c: &JsonCelle) -> Option<Celle> {
    let barn = || c.c.iter().map(fra_json).collect::<Option<Vec<_>>>();
    let innhold = match c.t.as_str() {
        "p" => Innhold::Pane(c.id.clone()?),
        "h" => Innhold::SideOmSide(barn()?),
        "v" => Innhold::OverHverandre(barn()?),
        _ => return None,
    };
    Some(Celle { bredde: c.w, høyde: c.h, innhold })
}

type Tegn<'a> = std::iter::Peekable<std::str::Chars<'a>>;

fn celle(tegn: &mut Tegn) -> Option<Celle> {
    let bredde = tall(tegn)?;
    (tegn.next()? == 'x').then_some(())?;
    let høyde = tall(tegn)?;
    for _ in 0..2 {
        (tegn.next()? == ',').then_some(())?;
        tall(tegn)?;
    }
    let innhold = match tegn.next()? {
        ',' => Innhold::Pane(format!("%{}", tall(tegn)?)),
        '{' => Innhold::SideOmSide(celler(tegn, '}')?),
        '[' => Innhold::OverHverandre(celler(tegn, ']')?),
        _ => return None,
    };
    Some(Celle { bredde, høyde, innhold })
}

fn celler(tegn: &mut Tegn, slutt: char) -> Option<Vec<Celle>> {
    let mut ut = vec![celle(tegn)?];
    loop {
        match tegn.next()? {
            ',' => ut.push(celle(tegn)?),
            c if c == slutt => return Some(ut),
            _ => return None,
        }
    }
}

fn tall(tegn: &mut Tegn) -> Option<u16> {
    let mut tekst = String::new();
    while let Some(c) = tegn.next_if(char::is_ascii_digit) {
        tekst.push(c);
    }
    tekst.parse().ok()
}

#[cfg(test)]
mod tester {
    use super::*;

    fn pane(id: &str, bredde: u16, høyde: u16) -> Celle {
        Celle { bredde, høyde, innhold: Innhold::Pane(id.into()) }
    }

    #[test]
    fn én_pane() {
        assert_eq!(tolk("b25d,80x24,0,0,3"), Some(pane("%3", 80, 24)));
    }

    #[test]
    fn side_om_side_med_to_over_hverandre_til_høyre() {
        let t = tolk("e5c1,159x48,0,0{79x48,0,0,1,79x48,80,0[79x24,80,0,2,79x23,80,25,3]}").unwrap();
        let høyre = Celle {
            bredde: 79,
            høyde: 48,
            innhold: Innhold::OverHverandre(vec![pane("%2", 79, 24), pane("%3", 79, 23)]),
        };
        assert_eq!(t.innhold, Innhold::SideOmSide(vec![pane("%1", 79, 48), høyre]));
    }

    #[test]
    fn json_fra_tmux_3_8() {
        let t = r#"{"V":2,"L":{"t":"h","w":120,"h":30,"x":0,"y":0,"c":[{"t":"p","w":60,"h":30,"x":0,"y":0,"l":1,"i":0,"I":"%0"},{"t":"v","w":59,"h":30,"x":61,"y":0,"c":[{"t":"p","w":59,"h":15,"x":61,"y":0,"l":0,"i":1,"I":"%1"},{"t":"p","w":59,"h":14,"x":61,"y":16,"a":true,"i":2,"I":"%2"}]}]}}"#;
        let høyre = Celle {
            bredde: 59,
            høyde: 30,
            innhold: Innhold::OverHverandre(vec![pane("%1", 59, 15), pane("%2", 59, 14)]),
        };
        assert_eq!(tolk(t).unwrap().innhold, Innhold::SideOmSide(vec![pane("%0", 60, 30), høyre]));
        assert_eq!(tolk(r#"{"V":2,"L":{"t":"p","w":120,"h":30,"I":"%1"}}"#), Some(pane("%1", 120, 30)));
    }

    #[test]
    fn ufullstendig_gir_ingenting() {
        assert_eq!(tolk("b25d,80x24,0,0"), None);
        assert_eq!(tolk("e5c1,159x48,0,0{79x48,0,0,1"), None);
        assert_eq!(tolk(""), None);
    }
}
