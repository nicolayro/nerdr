//! Repoet og branchen en mappe står i, lest fra `.git` uten å starte git.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Repo {
    /// Navnet på repoet. Et worktree får navnet til repoet det er laget fra,
    /// ikke sin egen mappe, som `claude -w` legger under `.claude/worktrees`.
    pub navn: String,
    /// Branchen, eller de sju første tegnene av commiten når HEAD ikke peker
    /// på en.
    pub branch: Option<String>,
    /// Mappa til repoet. Et worktree får mappa til repoet det er laget fra.
    pub rot: PathBuf,
}

pub fn les(mappe: &Path) -> Option<Repo> {
    let punkt_git = mappe.ancestors().map(|m| m.join(".git")).find(|g| g.exists())?;
    // I et worktree er `.git` en fil som peker til worktreets egen git-mappe,
    // `<repo>/.git/worktrees/<navn>`.
    let (git, felles) = if punkt_git.is_file() {
        let peker = fs::read_to_string(&punkt_git).ok()?;
        let sti = PathBuf::from(peker.strip_prefix("gitdir:")?.trim());
        let git = fs::canonicalize(punkt_git.parent()?.join(sti)).ok()?;
        let felles = match git.parent() {
            Some(p) if p.file_name()? == "worktrees" => p.parent()?.to_path_buf(),
            // En submodul, med git-mappa under `.git/modules`.
            _ => git.clone(),
        };
        (git, felles)
    } else {
        (punkt_git.clone(), punkt_git.clone())
    };
    let rot = match felles.file_name()? {
        n if n == ".git" => felles.parent()?.to_path_buf(),
        _ => punkt_git.parent()?.to_path_buf(),
    };
    let navn = match felles.file_name()? {
        n if n == ".git" => felles.parent()?.file_name()?,
        // Et bart repo, som `nerdr.git`.
        n => n,
    };
    let navn = navn.to_string_lossy();
    Some(Repo { navn: navn.strip_suffix(".git").unwrap_or(&navn).to_string(), branch: branch(&git), rot })
}

fn branch(git: &Path) -> Option<String> {
    let head = fs::read_to_string(git.join("HEAD")).ok()?;
    let head = head.trim();
    match head.strip_prefix("ref: refs/heads/") {
        Some(branch) => Some(branch.to_string()),
        None => head.get(..7).map(str::to_string),
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn leser_repo_og_branch_i_repo_worktree_og_løsrevet_head() {
        let rot = std::env::temp_dir().join(format!("nerdr-git-{}", std::process::id()));
        let repo = rot.join("repo");
        fs::create_dir_all(repo.join(".git/worktrees/wt")).unwrap();
        fs::create_dir_all(repo.join("app/src")).unwrap();
        fs::write(repo.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(repo.join(".git/worktrees/wt/HEAD"), "0123456789abcdef\n").unwrap();
        fs::create_dir_all(rot.join("wt")).unwrap();
        let peker = format!("gitdir: {}\n", repo.join(".git/worktrees/wt").display());
        fs::write(rot.join("wt/.git"), peker).unwrap();
        // Relativ peker, som med worktree.useRelativePaths.
        fs::create_dir_all(repo.join(".git/worktrees/rel")).unwrap();
        fs::write(repo.join(".git/worktrees/rel/HEAD"), "ref: refs/heads/fiks\n").unwrap();
        fs::create_dir_all(repo.join(".claude/worktrees/rel")).unwrap();
        fs::write(repo.join(".claude/worktrees/rel/.git"), "gitdir: ../../../.git/worktrees/rel\n").unwrap();

        let repo_og_branch = |m: &Path| les(m).map(|r| (r.navn, r.branch.unwrap_or_default()));
        assert_eq!(repo_og_branch(&repo.join("app/src")), Some(("repo".into(), "main".into())));
        assert_eq!(repo_og_branch(&rot.join("wt")), Some(("repo".into(), "0123456".into())));
        assert_eq!(repo_og_branch(&repo.join(".claude/worktrees/rel")), Some(("repo".into(), "fiks".into())));
        let rot_til = |m: &Path| les(m).and_then(|r| fs::canonicalize(r.rot).ok());
        let ekte = fs::canonicalize(&repo).ok();
        assert_eq!((rot_til(&repo.join("app/src")), rot_til(&rot.join("wt"))), (ekte.clone(), ekte));
        fs::remove_dir_all(rot).unwrap();
    }

    #[test]
    fn repo_i_en_mappe_som_heter_worktrees() {
        let rot = std::env::temp_dir().join(format!("nerdr-wt-{}", std::process::id()));
        fs::create_dir_all(rot.join("worktrees/.git")).unwrap();
        fs::write(rot.join("worktrees/.git/HEAD"), "ref: refs/heads/main\n").unwrap();
        assert_eq!(les(&rot.join("worktrees")).map(|r| r.navn).as_deref(), Some("worktrees"));
        fs::remove_dir_all(rot).unwrap();
    }

    #[test]
    fn bart_repo_uten_endelse() {
        let rot = std::env::temp_dir().join(format!("nerdr-bart-{}", std::process::id()));
        fs::create_dir_all(rot.join("nerdr.git/worktrees/wt")).unwrap();
        fs::write(rot.join("nerdr.git/worktrees/wt/HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::create_dir_all(rot.join("wt")).unwrap();
        fs::write(rot.join("wt/.git"), format!("gitdir: {}\n", rot.join("nerdr.git/worktrees/wt").display())).unwrap();
        assert_eq!(les(&rot.join("wt")).map(|r| r.navn).as_deref(), Some("nerdr"));
        fs::remove_dir_all(rot).unwrap();
    }
}
