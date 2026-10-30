//! Ny Claude: mappa, eller en tidligere samtale, velges i en popup, og
//! sesjonen i tmux får navnet til mappa.

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};
use std::{env, fs, mem};

use nerdr_kjerne::samtaler;

use crate::tekst::{mappenavn, med_tilde, varighet};

/// Mappene det søkes under, kolonseparert som `PATH`.
pub const MAPPER_VARIABEL: &str = "NERDR_MAPPER";
const STANDARD_MAPPER: &str = "~";
pub const SAMTALER: usize = 50;
/// En mappe med en av disse er et prosjekt, som en app i et monorepo.
const PROSJEKTFILER: [&str; 8] =
    ["package.json", "pom.xml", "build.gradle", "build.gradle.kts", "Cargo.toml", "go.mod", "pyproject.toml", "deno.json"];
/// Bygget, lastet ned eller testdata, og aldri et sted du starter en Claude.
const HOPP_OVER: [&str; 12] =
    ["node_modules", "target", "build", "dist", "out", "vendor", "__pycache__", "test", "tests", "__tests__", "fixtures", "mocks"];
/// Mappene til macOS i `$HOME`, med flere mapper enn alt annet der til sammen.
const SYSTEMMAPPER: [&str; 2] = ["Library", "Applications"];
/// Så mange nivåer under roten av et git-repo søkes det etter prosjekter.
/// Utenfor et repo er alle mappene med, men i et repo er det bare prosjektene,
/// for der er det mange flere mapper enn steder du starter en Claude.
const PROSJEKT_DYBDE: usize = 4;
/// Så ofte velgeren får mappene som er lest.
const SEND_HVER: Duration = Duration::from_millis(50);

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
    pub tegn: &'a [usize],
}

pub struct Velger {
    mapper: Vec<Kandidat>,
    lest: HashSet<PathBuf>,
    røtter: Vec<PathBuf>,
    samtaler: Vec<Kandidat>,
    søk: String,
    /// Poengene, plassen i kandidatene og tegnene søket traff, best først.
    /// Det er mange tusen mapper, så treffene regnes bare ut når søket endres.
    treff: Vec<(i32, usize, Vec<usize>)>,
    pub valgt: usize,
    /// Første treff som vises.
    pub rull: usize,
    /// Mappene under røttene er ikke lest ferdig.
    pub søker: bool,
    /// Tidligere samtaler vises i stedet for mappene.
    pub fortsett: bool,
    stopp: Arc<AtomicBool>,
}

impl Velger {
    /// `prosjekter` er `~/.claude/projects`, og samtalene i `kjører` er i gang
    /// og vises ikke. Mappene du har brukt Claude i, står først, så de vinner
    /// når treffet ellers er like godt.
    pub fn ny(fortsett: bool, prosjekter: &Path, kjører: &HashSet<String>) -> Velger {
        let hjem = PathBuf::from(env::var("HOME").unwrap_or_default());
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
        let mut velger = Velger {
            mapper: Vec::new(),
            lest: HashSet::new(),
            røtter: røtter(&mapper(), &hjem),
            samtaler,
            søk: String::new(),
            treff: Vec::new(),
            valgt: 0,
            rull: 0,
            søker: true,
            fortsett,
            stopp: Arc::new(AtomicBool::new(false)),
        };
        velger.legg_til(samtaler::mapper(prosjekter).into_iter().filter(|m| !skjult(m)).collect(), false);
        velger.finn_treff();
        velger
    }

    pub fn røtter(&self) -> Vec<PathBuf> {
        self.røtter.clone()
    }

    /// Settes når velgeren lukkes, så lesingen av mappene stopper.
    pub fn stopp(&self) -> Arc<AtomicBool> {
        self.stopp.clone()
    }

    /// Mapper fra en velger som er lukket, kan komme etter at en ny er åpnet.
    pub fn leser(&self, stopp: &Arc<AtomicBool>) -> bool {
        Arc::ptr_eq(&self.stopp, stopp)
    }

    /// Mappene kommer etter at velgeren er åpnet, så den du har valgt, blir
    /// stående valgt.
    pub fn legg_til(&mut self, mapper: Vec<PathBuf>, ferdig: bool) {
        let valgt = self.valgt().map(|k| k.mappe.clone());
        let fra = self.mapper.len();
        for mappe in mapper {
            if self.lest.insert(mappe.clone()) {
                self.mapper.push(Kandidat { vis: med_tilde(&mappe), mappe, samtale: None, alder: None });
            }
        }
        self.søker &= !ferdig;
        if self.fortsett || self.søk.is_empty() {
            return;
        }
        let nye = self.poeng_fra(fra);
        self.treff.extend(nye);
        self.treff.sort_by_key(|(poeng, i, _)| (-poeng, *i));
        if let Some(i) = valgt.and_then(|v| self.treff().iter().position(|t| t.kandidat.mappe == v)) {
            self.valgt = i;
        }
    }

    fn kandidater(&self) -> &[Kandidat] {
        if self.fortsett { &self.samtaler } else { &self.mapper }
    }

    pub fn antall(&self) -> usize {
        self.kandidater().len()
    }

    pub fn søk(&self) -> &str {
        &self.søk
    }

    pub fn bytt(&mut self) {
        self.fortsett = !self.fortsett;
        self.valgt = 0;
        self.finn_treff();
    }

    /// Mappene vises ikke før du skriver noe, for uten søk er det bare en lang
    /// liste i alfabetisk rekkefølge.
    fn finn_treff(&mut self) {
        self.treff = if self.søk.is_empty() && !self.fortsett { Vec::new() } else { self.poeng_fra(0) };
        self.treff.sort_by_key(|(poeng, i, _)| (-poeng, *i));
    }

    fn poeng_fra(&self, fra: usize) -> Vec<(i32, usize, Vec<usize>)> {
        self.kandidater()[fra..]
            .iter()
            .enumerate()
            .filter_map(|(i, k)| poeng_for(k, &self.søk).map(|(poeng, tegn)| (poeng, fra + i, tegn)))
            .collect()
    }

    /// Kandidatene søket treffer, best først. Like gode beholder rekkefølgen.
    pub fn treff(&self) -> Vec<Treff<'_>> {
        let kandidater = self.kandidater();
        self.treff.iter().map(|(_, i, tegn)| Treff { kandidat: &kandidater[*i], tegn }).collect()
    }

    pub fn antall_treff(&self) -> usize {
        self.treff.len()
    }

    pub fn skriv(&mut self, tekst: &str) {
        self.søk.extend(tekst.chars().filter(|c| !c.is_control()));
        self.valgt = 0;
        self.finn_treff();
    }

    pub fn slett(&mut self) {
        self.søk.pop();
        self.valgt = 0;
        self.finn_treff();
    }

    pub fn tøm(&mut self) {
        self.søk.clear();
        self.valgt = 0;
        self.finn_treff();
    }

    pub fn flytt(&mut self, steg: isize) {
        let siste = self.treff.len().saturating_sub(1) as isize;
        self.valgt = (self.valgt as isize + steg).clamp(0, siste) as usize;
    }

    pub fn valgt(&self) -> Option<&Kandidat> {
        self.treff.get(self.valgt).map(|(_, i, _)| &self.kandidater()[*i])
    }
}

impl Drop for Velger {
    fn drop(&mut self) {
        self.stopp.store(true, Ordering::Relaxed);
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

/// Et worktree i `.claude/worktrees` eller en mappe som `~/.config` er ikke
/// et sted du starter en ny Claude.
fn skjult(mappe: &Path) -> bool {
    mappe.components().any(|d| d.as_os_str().to_string_lossy().starts_with('.'))
}

/// Mappene under `røtter`, nivå for nivå, uten skjulte mapper og det i
/// `HOPP_OVER`. De sendes litt og litt, så velgeren kan brukes mens resten
/// leses, og `ferdig` er sann for de siste. Røttene selv er ikke med.
pub fn les_mapper(røtter: &[PathBuf], hjem: &Path, stopp: &AtomicBool, send: impl Fn(Vec<PathBuf>, bool)) {
    let systemmapper = SYSTEMMAPPER.map(|m| hjem.join(m));
    // Mappa, hvor dypt den ligger, og hvor dypt repoet den er i, starter.
    let mut kø: VecDeque<(PathBuf, usize, Option<usize>)> = røtter.iter().map(|r| (r.clone(), 0, None)).collect();
    let (mut lest, mut sendt) = (Vec::new(), Instant::now());
    while let Some((mappe, dybde, repo)) = kø.pop_front() {
        if stopp.load(Ordering::Relaxed) {
            return;
        }
        let Ok(filer) = fs::read_dir(&mappe) else { continue };
        let (mut under, mut prosjekt, mut git) = (Vec::new(), false, false);
        for fil in filer.flatten() {
            let (navn, Ok(type_)) = (fil.file_name(), fil.file_type()) else { continue };
            let navn = navn.to_string_lossy();
            if navn == ".git" {
                git = true;
            } else if type_.is_file() && PROSJEKTFILER.contains(&&*navn) {
                prosjekt = true;
            } else if type_.is_dir() && !navn.starts_with('.') && !HOPP_OVER.contains(&&*navn) {
                under.push(fil.path());
            }
        }
        let repo = repo.or(git.then_some(dybde));
        let i_repo = repo.map(|r| dybde - r);
        if dybde > 0 && i_repo.is_none_or(|d| d == 0 || prosjekt) {
            lest.push(mappe);
        }
        if i_repo.is_none_or(|d| d < PROSJEKT_DYBDE) {
            under.sort();
            kø.extend(under.into_iter().filter(|m| !systemmapper.contains(m)).map(|m| (m, dybde + 1, repo)));
        }
        if !lest.is_empty() && sendt.elapsed() >= SEND_HVER {
            send(mem::take(&mut lest), false);
            sendt = Instant::now();
        }
    }
    if !stopp.load(Ordering::Relaxed) {
        send(lest, true);
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
pub fn poeng(søk: &str, tekst: &str) -> Option<(i32, Vec<usize>)> {
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
    use std::sync::Mutex;

    #[test]
    fn alle_mappene_utenfor_et_repo_og_prosjektene_i_det() {
        let hjem = env::temp_dir().join(format!("nerdr-les-{}", std::process::id()));
        for (m, fil) in [
            ("notater/2026", "a.md"),
            ("Library/Caches", "a"),
            ("git/mono", ".git"),
            ("git/mono/apps/web", "package.json"),
            ("git/mono/docs", "README.md"),
            ("git/mono/apps/web/node_modules/pakke", "package.json"),
            ("git/mono/apps/web/tests/mock", "package.json"),
            ("git/mono/.skjult/app", "Cargo.toml"),
            ("git/mono/a/b/c/d/e", "go.mod"),
        ] {
            fs::create_dir_all(hjem.join(m)).unwrap();
            fs::write(hjem.join(m).join(fil), "").unwrap();
        }
        let (funnet, ferdig) = (Mutex::new(Vec::new()), AtomicBool::new(false));
        les_mapper(std::slice::from_ref(&hjem), &hjem, &AtomicBool::new(false), |mapper, siste| {
            funnet.lock().unwrap().extend(mapper);
            ferdig.store(siste, Ordering::Relaxed);
        });
        let funnet: Vec<_> = funnet.into_inner().unwrap().iter().map(|p| p.strip_prefix(&hjem).unwrap().to_path_buf()).collect();
        let nærmest_først = ["git", "notater", "git/mono", "notater/2026", "git/mono/apps/web"];
        assert_eq!(funnet, nærmest_først.map(PathBuf::from));
        assert!(ferdig.into_inner());
        fs::remove_dir_all(hjem).unwrap();
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
        let p = |vis: &str| poeng_for(&mappe(vis), "nettside").unwrap().0;
        assert!(p("~/git/mono/apps/nettside-blogg") > p("~/git/mono/apps/nettside-butikk/app"));
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
