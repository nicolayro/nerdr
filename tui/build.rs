//! Branchen nerdr er bygget fra, så du ser i appen at det ikke er main du
//! kjører. Uten git, eller fra en tag som i et release, er den tom.

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let ut = Command::new("git").args(args).output().ok().filter(|u| u.status.success())?;
    Some(String::from_utf8(ut.stdout).ok()?.trim().to_string())
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if let Some(head) = git(&["rev-parse", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={head}");
    }
    let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"]).filter(|b| b != "main" && b != "HEAD").unwrap_or_default();
    println!("cargo:rustc-env=NERDR_BRANCH={branch}");
}
