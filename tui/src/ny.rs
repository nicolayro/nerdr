//! Ny Claude: mappa, eller en tidligere samtale, velges i en popup, og
//! sesjonen i tmux får navnet til mappa.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::{env, fs};

use nerdr_kjerne::samtaler;

use crate::tekst::{mappenavn, med_tilde, varighet};

/// Mappene det søkes i, kolonseparert som `PATH`. Kandidatene er mappene rett
/// under dem.
pub const MAPPER_VARIABEL: &str = "NERDR_MAPPER";
const STANDARD_MAPPER: &str = "~/git";
pub const SAMTALER: usize = 50;
/// En mappe med en av disse er et prosjekt, som en app i et monorepo.
const PROSJEKTFILER: [&str; 8] =
    ["package.json", "pom.xml", "build.gradle", "build.gradle.kts", "Cargo.toml", "go.mod", "pyproject.toml", "deno.json"];
/// Bygget, lastet ned eller testdata, og aldri et sted du starter en Claude.
const HOPP_OVER: [&str; 12] =
    ["node_modules", "target", "build", "dist", "out", "vendor", "__pycache__", "test", "tests", "__tests__", "fixtures", "mocks"];
/// Så mange nivåer under mappene i `NERDR_MAPPER` søkes det etter prosjekter.
const PROSJEKT_DYBDE: usize = 4;

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
    toppmapper: Vec<PathBuf>,
    samtaler: Vec<Kandidat>,
    pub søk: String,
    pub valgt: usize,
    /// Prosjektene under mappene er ikke lest ennå.
    pub søker: bool,
    /// Tidligere samtaler vises i stedet for mappene.
    pub fortsett: bool,
}

impl Velger {
    /// `prosjekter` er `~/.claude/projects`, og samtalene i `kjører` er i gang
    /// og vises ikke. Mappene du har brukt Claude i, står først, så velgeren
    /// er til nytte uten at `NERDR_MAPPER` er satt.
    pub fn ny(fortsett: bool, prosjekter: &Path, kjører: &HashSet<String>) -> Velger {
        let hjem = PathBuf::from(env::var("HOME").unwrap_or_default());
        let toppmapper = kandidater(&røtter(&mapper(), &hjem));
        let mut mapper = Vec::new();
        let brukte = samtaler::mapper(prosjekter).into_iter().filter(|m| !skjult(m));
        for mappe in brukte.chain(toppmapper.iter().cloned()) {
            if !mapper.iter().any(|k: &Kandidat| k.mappe == mappe) {
                mapper.push(Kandidat { vis: med_tilde(&mappe), mappe, samtale: None, alder: None });
            }
        }
        let nå = SystemTime::now();
        let samtaler = samtaler::siste(prosjekter, SAMTALER, kjører)
            .into_iter()
            .map(|s| {
                Kandidat {
                    vis: format!("{}  {}", mappenavn(&s.mappe), s.tittel),
                    alder: Some(varighet(nå.duration_since(s.endret).unwrap_or_default().as_millis() as u64)),
                    samtale: Some(s.id),
                    mappe: s.mappe,
                }
            })
            .collect();
        Velger { mapper, toppmapper, samtaler, søk: String::new(), valgt: 0, søker: true, fortsett }
    }

    /// Mappene rett under dem i `NERDR_MAPPER`, der prosjektene søkes. Ikke
    /// mappene du har brukt Claude i, for en av dem kan være `~`.
    pub fn toppmapper(&self) -> Vec<PathBuf> {
        self.toppmapper.clone()
    }

    /// Prosjektene kommer etter at velgeren er åpnet, så den du har valgt,
    /// blir stående valgt.
    pub fn legg_til(&mut self, prosjekter: Vec<PathBuf>) {
        let valgt = self.valgt().map(|k| k.mappe.clone());
        let nye: Vec<PathBuf> = prosjekter.into_iter().filter(|p| !self.mapper.iter().any(|k| &k.mappe == p)).collect();
        self.mapper.extend(nye.into_iter().map(|mappe| Kandidat { vis: med_tilde(&mappe), mappe, samtale: None, alder: None }));
        self.søker = false;
        if !self.fortsett
            && let Some(i) = valgt.and_then(|v| self.treff().iter().position(|t| t.kandidat.mappe == v))
        {
            self.valgt = i;
        }
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
                let (poeng, tegn) = poeng_for(k, &self.søk)?;
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

/// En worktree i `.claude/worktrees` eller en mappe som `~/.config` er ikke
/// et sted du starter en ny Claude.
fn skjult(mappe: &Path) -> bool {
    mappe.components().any(|d| d.as_os_str().to_string_lossy().starts_with('.'))
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

/// Prosjektene under `mapper`, nivå for nivå, uten skjulte mapper og det i
/// `HOPP_OVER`. Mappene selv er ikke med.
pub fn prosjekter(mapper: &[PathBuf]) -> Vec<PathBuf> {
    let mut ut = Vec::new();
    for mappe in mapper {
        let_etter_prosjekter(mappe, 0, &mut ut);
    }
    ut
}

fn let_etter_prosjekter(mappe: &Path, dybde: usize, ut: &mut Vec<PathBuf>) {
    let Ok(filer) = fs::read_dir(mappe) else { return };
    let (mut under, mut prosjekt) = (Vec::new(), false);
    for fil in filer.flatten() {
        let (navn, Ok(type_)) = (fil.file_name(), fil.file_type()) else { continue };
        let navn = navn.to_string_lossy();
        if type_.is_file() && PROSJEKTFILER.contains(&&*navn) {
            prosjekt = true;
        } else if type_.is_dir() && !navn.starts_with('.') && !HOPP_OVER.contains(&&*navn) {
            under.push(fil.path());
        }
    }
    if prosjekt && dybde > 0 {
        ut.push(mappe.to_path_buf());
    }
    if dybde < PROSJEKT_DYBDE {
        under.sort();
        for mappe in under {
            let_etter_prosjekter(&mappe, dybde + 1, ut);
        }
    }
}

/// En mappe der søket treffer i siste del av stien, er den du leter etter, og
/// står foran mappene under den, som `app` og `web` inne i en app.
fn poeng_for(k: &Kandidat, søk: &str) -> Option<(i32, Vec<usize>)> {
    let (poeng, tegn) = poeng(søk, &k.vis)?;
    let siste_del = k.vis.rfind('/').map_or(0, |i| k.vis[..i].chars().count() + 1);
    let i_siste = k.samtale.is_none() && tegn.first().is_some_and(|&i| i >= siste_del);
    Some((poeng + if i_siste { 20 } else { 0 }, tegn))
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

#[cfg(test)]
mod tester {
    use super::*;

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
    fn prosjekter_under_mappene_uten_det_som_er_bygget() {
        let rot = env::temp_dir().join(format!("nerdr-prosjekter-{}", std::process::id()));
        for (m, fil) in [
            ("mono", "package.json"),
            ("mono/apps/web", "package.json"),
            ("mono/libs/felles", "pom.xml"),
            ("mono/apps/web/node_modules/pakke", "package.json"),
            ("mono/apps/web/tests/mock", "package.json"),
            ("mono/.skjult/app", "Cargo.toml"),
            ("mono/a/b/c/d/e", "go.mod"),
            ("mono/docs", "README.md"),
        ] {
            fs::create_dir_all(rot.join(m)).unwrap();
            fs::write(rot.join(m).join(fil), "").unwrap();
        }
        let funnet: Vec<_> = prosjekter(&[rot.join("mono")]).iter().map(|p| p.strip_prefix(&rot).unwrap().to_path_buf()).collect();
        assert_eq!(funnet, [Path::new("mono/apps/web"), Path::new("mono/libs/felles")]);
        fs::remove_dir_all(rot).unwrap();
    }

    #[test]
    fn røtter_fra_hjem_med_eller_uten_tilde() {
        let hjem = Path::new("/h");
        let røtter = røtter("~/git: kode ::/abs/sti:~", hjem);
        assert_eq!(røtter, [Path::new("/h/git"), Path::new("/h/kode"), Path::new("/abs/sti"), Path::new("/h")]);
    }

    #[test]
    fn treff_i_siste_del_av_stien_står_først() {
        let mappe = |vis: &str| Kandidat { mappe: PathBuf::from(vis), vis: vis.into(), samtale: None, alder: None };
        let p = |vis: &str| poeng_for(&mappe(vis), "kontorfront").unwrap().0;
        assert!(p("~/git/awl/apps/kontorfront-skjema") > p("~/git/awl/apps/kontorfront-fond/app"));
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
