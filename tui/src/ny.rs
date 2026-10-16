//! Ny Claude: mappa, eller en tidligere samtale, velges i en popup, og
//! sesjonen i tmux får navnet til mappa.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use std::{env, fs};

use nerdr_kjerne::samtaler;

/// Mappene det søkes i, kolonseparert som `PATH`. Kandidatene er mappene rett
/// under dem.
pub const MAPPER_VARIABEL: &str = "NERDR_MAPPER";
const STANDARD_MAPPER: &str = "~/git";
const SAMTALER: usize = 50;

pub struct Kandidat {
    pub mappe: PathBuf,
    /// Det som vises og søkes i: stien med `~` for `$HOME`, eller mappenavnet
    /// og tittelen til en samtale.
    pub vis: String,
    /// Samtalen `claude --resume` skal fortsette.
    pub samtale: Option<String>,
    /// Hvor lenge siden samtalen sist ble endret.
    pub alder: Option<String>,
}

pub struct Treff<'a> {
    pub kandidat: &'a Kandidat,
    /// Tegnindeksene i `vis` som søket traff.
    pub tegn: Vec<usize>,
}

pub struct Velger {
    mapper: Vec<Kandidat>,
    samtaler: Vec<Kandidat>,
    pub søk: String,
    pub valgt: usize,
    /// Tidligere samtaler vises i stedet for mappene.
    pub fortsett: bool,
}

impl Velger {
    /// `prosjekter` er `~/.claude/projects`, og samtalene i `kjører` er i gang
    /// og vises ikke.
    pub fn ny(fortsett: bool, prosjekter: &Path, kjører: &HashSet<String>) -> Velger {
        let hjem = PathBuf::from(env::var("HOME").unwrap_or_default());
        let mapper = kandidater(&røtter(&mapper(), &hjem))
            .into_iter()
            .map(|mappe| {
                let vis = mappe
                    .strip_prefix(&hjem)
                    .map_or_else(|_| mappe.display().to_string(), |r| format!("~/{}", r.display()));
                Kandidat { mappe, vis, samtale: None, alder: None }
            })
            .collect();
        let nå = SystemTime::now();
        let samtaler = samtaler::siste(prosjekter, SAMTALER, kjører)
            .into_iter()
            .map(|s| {
                let navn = s.mappe.file_name().map_or_else(|| s.mappe.display().to_string(), |n| n.to_string_lossy().into());
                Kandidat {
                    vis: format!("{navn}  {}", s.tittel),
                    alder: Some(alder(nå.duration_since(s.endret).unwrap_or_default())),
                    samtale: Some(s.id),
                    mappe: s.mappe,
                }
            })
            .collect();
        Velger { mapper, samtaler, søk: String::new(), valgt: 0, fortsett }
    }

    fn kandidater(&self) -> &[Kandidat] {
        if self.fortsett { &self.samtaler } else { &self.mapper }
    }

    pub fn antall(&self) -> usize {
        self.kandidater().len()
    }

    pub fn bytt(&mut self) {
        self.fortsett = !self.fortsett;
        self.valgt = 0;
    }

    /// Kandidatene søket treffer, best først. Like gode beholder rekkefølgen.
    pub fn treff(&self) -> Vec<Treff<'_>> {
        let mut treff: Vec<(i32, Treff)> = self
            .kandidater()
            .iter()
            .filter_map(|k| {
                let (poeng, tegn) = poeng(&self.søk, &k.vis)?;
                Some((poeng, Treff { kandidat: k, tegn }))
            })
            .collect();
        treff.sort_by_key(|(poeng, _)| -poeng);
        treff.into_iter().map(|(_, t)| t).collect()
    }

    pub fn skriv(&mut self, tekst: &str) {
        self.søk.extend(tekst.chars().filter(|c| !c.is_control()));
        self.valgt = 0;
    }

    pub fn slett(&mut self) {
        self.søk.pop();
        self.valgt = 0;
    }

    pub fn flytt(&mut self, steg: isize) {
        let siste = self.treff().len().saturating_sub(1) as isize;
        self.valgt = (self.valgt as isize + steg).clamp(0, siste) as usize;
    }

    pub fn valgt(&self) -> Option<&Kandidat> {
        self.treff().into_iter().nth(self.valgt).map(|t| t.kandidat)
    }
}

fn alder(tid: Duration) -> String {
    match tid.as_secs() / 60 {
        0 => "nå".into(),
        m @ 1..60 => format!("{m} min"),
        m @ 60..1440 => format!("{} t", m / 60),
        m => format!("{} d", m / 1440),
    }
}

/// Verdien av `NERDR_MAPPER`, eller standarden når den ikke er satt.
pub fn mapper() -> String {
    env::var(MAPPER_VARIABEL).ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| STANDARD_MAPPER.into())
}

/// Stier som ikke er absolutte, regnes fra `hjem`, med eller uten `~/` foran.
fn røtter(mapper: &str, hjem: &Path) -> Vec<PathBuf> {
    mapper
        .split(':')
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(|m| hjem.join(m.strip_prefix('~').map_or(m, |r| r.trim_start_matches('/'))))
        .collect()
}

fn kandidater(røtter: &[PathBuf]) -> Vec<PathBuf> {
    let mut alle = Vec::new();
    for rot in røtter {
        let Ok(filer) = fs::read_dir(rot) else { continue };
        let mut mapper: Vec<PathBuf> = filer
            .flatten()
            .filter(|f| !f.file_name().to_string_lossy().starts_with('.'))
            .map(|f| f.path())
            .filter(|m| m.is_dir())
            .collect();
        mapper.sort();
        alle.extend(mapper);
    }
    alle
}

/// Treffer når bokstavene i `søk` står i samme rekkefølge i `tekst`, uten å
/// skille store og små. Bokstaver som står inntil hverandre og bokstaver først i
/// et ord gir mer, så `ner` velger `nerdr` framfor `annet-er`. Hver
/// startposisjon prøves, siden det første treffet ikke alltid er det beste.
fn poeng(søk: &str, tekst: &str) -> Option<(i32, Vec<usize>)> {
    let søk: Vec<char> = søk.to_lowercase().chars().collect();
    let tekst: Vec<char> = tekst.to_lowercase().chars().collect();
    let Some(&første) = søk.first() else { return Some((0, Vec::new())) };
    let ordstart = |i: usize| i == 0 || matches!(tekst[i - 1], '/' | '-' | '_' | '.' | ' ');

    let mut best: Option<(i32, Vec<usize>)> = None;
    for start in (0..tekst.len()).filter(|&i| tekst[i] == første) {
        let mut tegn = vec![start];
        for &c in &søk[1..] {
            let fra = tegn.last().unwrap() + 1;
            match (fra..tekst.len()).find(|&i| tekst[i] == c) {
                Some(i) => tegn.push(i),
                None => break,
            }
        }
        if tegn.len() < søk.len() {
            break;
        }
        let poeng = tegn
            .iter()
            .enumerate()
            .map(|(n, &i)| 1 + if ordstart(i) { 8 } else { 0 } + if n > 0 && tegn[n - 1] + 1 == i { 5 } else { 0 })
            .sum::<i32>()
            - tekst.len() as i32 / 8;
        if best.as_ref().is_none_or(|(b, _)| poeng > *b) {
            best = Some((poeng, tegn));
        }
    }
    best
}

/// Punktum og kolon er skilletegn i tmux-targets, og tmux bytter dem ut selv.
/// Da ville navnet ikke funnet sesjonen igjen.
pub fn sesjonsnavn(mappe: &Path) -> String {
    let navn = mappe.file_name().map_or_else(|| "claude".into(), |n| n.to_string_lossy());
    navn.replace(['.', ':'], "-")
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn sesjonsnavn_uten_skilletegn() {
        assert_eq!(sesjonsnavn(Path::new("/h/git/nit.æ")), "nit-æ");
        assert_eq!(sesjonsnavn(Path::new("/h/git/a:b")), "a-b");
        assert_eq!(sesjonsnavn(Path::new("/")), "claude");
    }

    #[test]
    fn kandidater_er_mappene_rett_under_hver_rot() {
        let hjem = env::temp_dir().join(format!("nerdr-ny-{}", std::process::id()));
        for m in ["git/b", "git/a/dypere", "git/.skjult", "kode/c"] {
            fs::create_dir_all(hjem.join(m)).unwrap();
        }
        fs::write(hjem.join("git/fil"), "").unwrap();

        let navn: Vec<_> = kandidater(&røtter("git:kode", &hjem)).iter().map(|m| m.strip_prefix(&hjem).unwrap().to_path_buf()).collect();
        assert_eq!(navn, [Path::new("git/a"), Path::new("git/b"), Path::new("kode/c")]);
        fs::remove_dir_all(hjem).unwrap();
    }

    #[test]
    fn alder_i_minutter_timer_og_dager() {
        let min = |m: u64| alder(Duration::from_secs(m * 60));
        assert_eq!([min(0), min(5), min(90), min(3 * 1440)], ["nå", "5 min", "1 t", "3 d"]);
    }

    #[test]
    fn røtter_fra_hjem_med_eller_uten_tilde() {
        let hjem = Path::new("/h");
        let røtter = røtter("~/git: kode ::/abs/sti:~", hjem);
        assert_eq!(røtter, [Path::new("/h/git"), Path::new("/h/kode"), Path::new("/abs/sti"), Path::new("/h")]);
    }

    #[test]
    fn bokstavene_i_rekkefølge_uten_å_skille_store_og_små() {
        assert_eq!(poeng("NRD", "~/git/nerdr").map(|(_, t)| t), Some(vec![6, 8, 9]));
        assert_eq!(poeng("rn", "~/git/nerdr"), None);
        assert!(poeng("", "hva som helst").is_some());
    }

    #[test]
    fn ordstart_og_sammenhengende_bokstaver_vinner() {
        let p = |søk, tekst| poeng(søk, tekst).unwrap().0;
        assert!(p("ner", "~/kode/nerdr") > p("ner", "~/git/annet-er"));
        // Første n står i «annet», men det beste treffet er i «nerdr».
        assert_eq!(poeng("nerdr", "~/git/annet-nerdr").map(|(_, t)| t[0]), Some(12));
    }
}
