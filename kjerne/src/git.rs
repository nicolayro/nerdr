//! Branchen en mappe står på, lest fra `.git/HEAD` uten å starte git.

use std::fs;
use std::path::{Path, PathBuf};

/// Branchen, eller de sju første tegnene av commiten når HEAD ikke peker på en.
pub fn branch(mappe: &Path) -> Option<String> {
    let git = mappe.ancestors().map(|m| m.join(".git")).find(|g| g.exists())?;
    // I en worktree er `.git` en fil som peker til worktreens egen git-mappe.
    let git = if git.is_file() {
        let peker = fs::read_to_string(&git).ok()?;
        let sti = PathBuf::from(peker.strip_prefix("gitdir:")?.trim());
        git.parent()?.join(sti)
    } else {
        git
    };
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
    fn leser_branch_i_repo_worktree_og_løsrevet_head() {
        let rot = std::env::temp_dir().join(format!("nerdr-git-{}", std::process::id()));
        let repo = rot.join("repo");
        fs::create_dir_all(repo.join(".git/worktrees/wt")).unwrap();
        fs::create_dir_all(repo.join("app/src")).unwrap();
        fs::write(repo.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(repo.join(".git/worktrees/wt/HEAD"), "0123456789abcdef\n").unwrap();
        fs::create_dir_all(rot.join("wt")).unwrap();
        let peker = format!("gitdir: {}\n", repo.join(".git/worktrees/wt").display());
        fs::write(rot.join("wt/.git"), peker).unwrap();

        assert_eq!(branch(&repo.join("app/src")).as_deref(), Some("main"));
        assert_eq!(branch(&rot.join("wt")).as_deref(), Some("0123456"));
        fs::remove_dir_all(rot).unwrap();
    }
}
