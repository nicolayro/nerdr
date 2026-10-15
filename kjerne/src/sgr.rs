//! Tolker utdata fra `capture-pane -e`: tekst med SGR-sekvenser for farger og
//! attributter.
//!
//! tmux bærer stilen videre over linjeskift og nullstiller den bare når den
//! endres, så tolkningen må ta alle linjene i ett løp.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Farge {
    #[default]
    Standard,
    Indeks(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stil {
    pub fg: Farge,
    pub bg: Farge,
    pub fet: bool,
    pub svak: bool,
    pub kursiv: bool,
    pub understrek: bool,
    pub blink: bool,
    pub invers: bool,
    pub skjult: bool,
    pub overstrek: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bit {
    pub tekst: String,
    pub stil: Stil,
}

pub type Linje = Vec<Bit>;

pub fn tolk(linjer: &[String]) -> Vec<Linje> {
    let mut stil = Stil::default();
    linjer.iter().map(|l| tolk_linje(l, &mut stil)).collect()
}

fn tolk_linje(linje: &str, stil: &mut Stil) -> Linje {
    let mut ut: Linje = Vec::new();
    let mut tekst = String::new();
    let mut tegn = linje.chars().peekable();

    while let Some(c) = tegn.next() {
        if c != '\x1b' {
            if !c.is_control() {
                tekst.push(c);
            }
            continue;
        }
        match tegn.next() {
            Some('[') => {
                let mut param = String::new();
                let mut slutt = None;
                for c in tegn.by_ref() {
                    if ('\x40'..='\x7e').contains(&c) {
                        slutt = Some(c);
                        break;
                    }
                    param.push(c);
                }
                if slutt == Some('m') {
                    if !tekst.is_empty() {
                        ut.push(Bit { tekst: std::mem::take(&mut tekst), stil: *stil });
                    }
                    bruk(&param, stil);
                }
            }
            // OSC, som hyperlenker: hoppes over til BEL eller ESC \.
            Some(']') => {
                while let Some(c) = tegn.next() {
                    if c == '\x07' || (c == '\x1b' && tegn.next_if_eq(&'\\').is_some()) {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    if !tekst.is_empty() {
        ut.push(Bit { tekst, stil: *stil });
    }
    ut
}

fn bruk(param: &str, stil: &mut Stil) {
    let mut deler = param.split(';');
    while let Some(del) = deler.next() {
        let mut sub = del.split(':');
        let kode: u16 = sub.next().unwrap_or("").parse().unwrap_or(0);
        match kode {
            0 => *stil = Stil::default(),
            1 => stil.fet = true,
            2 => stil.svak = true,
            3 => stil.kursiv = true,
            4 => stil.understrek = sub.next() != Some("0"),
            5 => stil.blink = true,
            7 => stil.invers = true,
            8 => stil.skjult = true,
            9 => stil.overstrek = true,
            21 => stil.understrek = true,
            22 => (stil.fet, stil.svak) = (false, false),
            23 => stil.kursiv = false,
            24 => stil.understrek = false,
            25 => stil.blink = false,
            27 => stil.invers = false,
            28 => stil.skjult = false,
            29 => stil.overstrek = false,
            30..=37 => stil.fg = Farge::Indeks((kode - 30) as u8),
            39 => stil.fg = Farge::Standard,
            40..=47 => stil.bg = Farge::Indeks((kode - 40) as u8),
            49 => stil.bg = Farge::Standard,
            90..=97 => stil.fg = Farge::Indeks((kode - 90 + 8) as u8),
            100..=107 => stil.bg = Farge::Indeks((kode - 100 + 8) as u8),
            38 | 48 | 58 => {
                let sub: Vec<&str> = sub.collect();
                let farge = match sub.as_slice() {
                    // Semikolonformen, `38;5;n` og `38;2;r;g;b`, sprer fargen
                    // over leddene etter.
                    [] => utvidet_farge(&mut deler),
                    // Kolonformen har alt i ett ledd, med et valgfritt fargerom
                    // foran rgb: `38:2::r:g:b`.
                    ["5", n] => utvidet_farge(&mut ["5", n].into_iter()),
                    ["2", .., r, g, b] => utvidet_farge(&mut ["2", r, g, b].into_iter()),
                    _ => None,
                };
                match (kode, farge) {
                    (38, Some(f)) => stil.fg = f,
                    (48, Some(f)) => stil.bg = f,
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn utvidet_farge<'a>(deler: &mut impl Iterator<Item = &'a str>) -> Option<Farge> {
    let mut tall = || deler.next().and_then(|d| d.parse::<u8>().ok());
    match tall()? {
        5 => Some(Farge::Indeks(tall()?)),
        2 => Some(Farge::Rgb(tall()?, tall()?, tall()?)),
        _ => None,
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    fn en(linje: &str) -> Linje {
        tolk(&[linje.to_string()]).remove(0)
    }

    #[test]
    fn farger_og_attributter() {
        let l = en("\x1b[38;5;208mA\x1b[38;2;10;20;30m\x1b[48;5;4mB\x1b[39m\x1b[49m C\x1b[3;4mD\x1b[0m");
        assert_eq!(l[0].stil.fg, Farge::Indeks(208));
        assert_eq!((l[1].stil.fg, l[1].stil.bg), (Farge::Rgb(10, 20, 30), Farge::Indeks(4)));
        assert_eq!(l[2].stil, Stil::default());
        assert_eq!(l[2].tekst, " C");
        assert!(l[3].stil.kursiv && l[3].stil.understrek);
    }

    #[test]
    fn stilen_bæres_over_linjeskift() {
        let linjer = tolk(&["\x1b[1mfet".to_string(), "fortsatt".to_string()]);
        assert!(linjer[1][0].stil.fet);
    }

    #[test]
    fn kolonform_og_osc_hoppes_riktig() {
        let l = en("\x1b]8;;https://x\x1b\\lenke\x1b]8;;\x07 \x1b[38:2::1:2:3mX");
        assert_eq!(l[0].tekst, "lenke ");
        assert_eq!(l[1].stil.fg, Farge::Rgb(1, 2, 3));
    }

    #[test]
    fn brede_tegn_beholdes() {
        assert_eq!(en("中文 rød")[0].tekst, "中文 rød");
    }
}
