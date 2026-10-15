//! Repo, worktree og branch for en katalog, funnet ved å lese filer.
//!
//! Git startes aldri. På maskiner med Defender og Jamf Protect koster hver
//! prosessoppstart 400–900 ms under last, mens de samme opplysningene ligger i
//! to småfiler som leses på mikrosekunder.

use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, Default)]
struct Funn {
    /// Worktreets egen gitdir; lik `felles` utenfor et worktree.
    gitdir: PathBuf,
    /// Hovedrepoets `.git`.
    felles: PathBuf,
    /// Katalogen `.git` ble funnet i, altså utsjekken du står i.
    rot: PathBuf,
}

#[derive(Default)]
pub struct Git {
    /// Uten utløp: hvilken gitdir en katalog hører til, endrer seg ikke.
    cache: HashMap<String, Option<Funn>>,
}

impl Git {
    fn funn(&mut self, sti: &str) -> Option<&Funn> {
        self.cache
            .entry(sti.to_string())
            .or_insert_with(|| finn(Path::new(sti)))
            .as_ref()
    }

    pub fn repo(&mut self, sti: &str) -> String {
        match self.funn(sti) {
            Some(f) => navn(f.felles.parent().unwrap_or(&f.felles)),
            None => navn(Path::new(sti)),
        }
    }

    /// Det som skiller denne utsjekken fra repoet selv, ellers tom streng.
    ///
    /// Repo-navnet står i gruppeoverskriften, så det strippes som prefiks. Er
    /// utsjekken navngitt likt repoet, er den hovedutsjekken og trenger ingen
    /// merkelapp.
    pub fn worktree(&mut self, sti: &str, repo: &str) -> String {
        let Some(f) = self.funn(sti) else { return String::new() };
        let utsjekk = navn(&f.rot);
        if utsjekk == repo {
            return String::new();
        }
        match utsjekk.strip_prefix(&format!("{repo}-")) {
            Some(rest) => rest.to_string(),
            None => utsjekk,
        }
    }

    /// Lest fra HEAD hver gang, så den er fersk uten TTL. Detached HEAD gir
    /// tom streng, som `git branch --show-current`.
    pub fn branch(&mut self, sti: &str) -> String {
        let Some(f) = self.funn(sti) else { return String::new() };
        les(&f.gitdir.join("HEAD"))
            .strip_prefix("ref: refs/heads/")
            .map(str::to_string)
            .unwrap_or_default()
    }
}

fn finn(sti: &Path) -> Option<Funn> {
    let mut kat = normaliser(sti);
    loop {
        let peker = kat.join(".git");
        if peker.is_dir() {
            return Some(Funn { gitdir: peker.clone(), felles: peker, rot: kat });
        }
        if peker.is_file() {
            let gitdir = les(&peker).strip_prefix("gitdir:")?.trim().to_string();
            let gitdir = normaliser(&kat.join(gitdir));
            // commondir står i worktreets gitdir og peker på hovedrepoets .git.
            let felles = match les(&gitdir.join("commondir")).as_str() {
                "" => gitdir.clone(),
                rel => normaliser(&gitdir.join(rel)),
            };
            return Some(Funn { gitdir, felles, rot: kat });
        }
        if !kat.pop() {
            return None;
        }
    }
}

fn les(sti: &Path) -> String {
    fs::read_to_string(sti).map(|t| t.trim().to_string()).unwrap_or_default()
}

fn navn(sti: &Path) -> String {
    sti.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string()
}

/// `..` og `.` løses rent tekstlig, som `os.path.normpath`, uten å følge lenker.
fn normaliser(sti: &Path) -> PathBuf {
    let mut ut = PathBuf::new();
    for del in sti.components() {
        match del {
            Component::CurDir => {}
            Component::ParentDir => {
                ut.pop();
            }
            annen => ut.push(annen),
        }
    }
    ut
}

#[cfg(test)]
mod tester {
    use super::*;

    fn tmp(navn: &str) -> PathBuf {
        let rot = std::env::temp_dir().join(format!("nerdr-git-{navn}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&rot);
        rot
    }

    #[test]
    fn hovedutsjekk() {
        let rot = tmp("hoved");
        let repo = rot.join("awl");
        fs::create_dir_all(repo.join(".git")).unwrap();
        fs::create_dir_all(repo.join("app/src")).unwrap();
        fs::write(repo.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();

        let mut git = Git::default();
        let sti = repo.join("app/src").to_string_lossy().to_string();
        assert_eq!(git.repo(&sti), "awl");
        assert_eq!(git.worktree(&sti, "awl"), "");
        assert_eq!(git.branch(&sti), "main");
        fs::remove_dir_all(rot).unwrap();
    }

    #[test]
    fn worktree_med_repoprefiks() {
        let rot = tmp("wt");
        let felles = rot.join("awl/.git");
        let wt_gitdir = felles.join("worktrees/awl-feature-1");
        fs::create_dir_all(&wt_gitdir).unwrap();
        fs::write(wt_gitdir.join("commondir"), "../..\n").unwrap();
        fs::write(wt_gitdir.join("HEAD"), "ref: refs/heads/ny-ting\n").unwrap();
        let wt = rot.join("awl-feature-1");
        fs::create_dir_all(&wt).unwrap();
        fs::write(wt.join(".git"), format!("gitdir: {}\n", wt_gitdir.display())).unwrap();

        let mut git = Git::default();
        let sti = wt.to_string_lossy().to_string();
        assert_eq!(git.repo(&sti), "awl");
        assert_eq!(git.worktree(&sti, "awl"), "feature-1");
        assert_eq!(git.branch(&sti), "ny-ting");
        fs::remove_dir_all(rot).unwrap();
    }

    #[test]
    fn utenfor_repo() {
        let mut git = Git::default();
        assert_eq!(git.repo("/"), "");
        assert_eq!(git.branch("/"), "");
    }
}
