#!/usr/bin/env rust-script

//! ```cargo
//! [dependencies]
//! serde = { version = "1", features = ["derive"] }
//! serde_json = "1"
//! ```

use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::{env, fs};

#[derive(Deserialize)]
struct Repo {
    name: String,
    #[serde(rename = "isPrivate")]
    is_private: bool,
}

fn resolve_root() -> PathBuf {
    env::var("MIRROR_GALLERY_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").expect("HOME is not set");
            PathBuf::from(home).join("Mirrors").join("Github")
        })
}

fn preflight_gh_auth() -> Result<String, String> {
    let out = Command::new("gh")
        .args(["auth", "status", "--hostname", "github.com"])
        .output()
        .map_err(|e| format!("cannot run gh: {e}"))?;

    if out.status.success() {
        let text = String::from_utf8_lossy(&out.stdout);
        let needle = "Logged in to github.com account ";
        let user = text
            .lines()
            .find_map(|l| {
                l.find(needle)
                    .map(|pos| &l[pos + needle.len()..])
                    .and_then(|rest| rest.split_whitespace().next())
                    .map(String::from)
            })
            .unwrap_or_else(|| "unknown".into());
        Ok(user)
    } else {
        let combined = [
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ]
        .join("\n");
        Err(combined)
    }
}

fn gh_list_repos(owner: &str) -> Result<Vec<Repo>, String> {
    let out = Command::new("gh")
        .args(["repo", "list", owner, "--limit", "10000", "--json", "name,isPrivate"])
        .output()
        .map_err(|e| format!("gh: {e}"))?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }

    serde_json::from_slice(&out.stdout).map_err(|e| format!("parse: {e}"))
}

fn sync_repo(owner: &str, repo: &Repo, target: &Path) -> bool {
    let vis = if repo.is_private { " (private)" } else { "" };

    if target.exists() {
        eprint!("  [fetch] {}/{}{vis} ", owner, repo.name);
        match Command::new("git")
            .args(["-C", &target.to_string_lossy(), "fetch", "--all", "--prune"])
            .output()
        {
            Ok(o) if o.status.success() => { eprintln!("ok"); true }
            Ok(o) => { eprintln!("FAIL (exit {})", o.status); false }
            Err(e) => { eprintln!("FAIL ({e})"); false }
        }
    } else {
        eprint!("  [clone] {}/{}{vis} ", owner, repo.name);
        match Command::new("gh")
            .args([
                "repo", "clone",
                &format!("{}/{}", owner, repo.name),
                &target.to_string_lossy(),
            ])
            .output()
        {
            Ok(o) if o.status.success() => { eprintln!("ok"); true }
            Ok(o) => {
                eprintln!("FAIL ({})", String::from_utf8_lossy(&o.stderr).trim());
                false
            }
            Err(e) => { eprintln!("FAIL ({e})"); false }
        }
    }
}

fn mirror_owner(root: &Path, owner: &str) -> (usize, usize) {
    eprintln!("[{owner}] listing repositories...");

    let repos = match gh_list_repos(owner) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[{owner}] {e}");
            return (0, 0);
        }
    };

    let (public, private) = repos.iter().fold((0usize, 0usize), |(pu, pr), r| {
        if r.is_private { (pu, pr + 1) } else { (pu + 1, pr) }
    });
    eprintln!("[{owner}] {public} public, {private} private");

    let dir = root.join(owner);
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("[{owner}] cannot create directory: {e}");
        return (0, 0);
    }

    repos
        .iter()
        .map(|repo| sync_repo(owner, repo, &dir.join(&repo.name)))
        .fold((0usize, 0usize), |(ok, fail), success| {
            if success { (ok + 1, fail) } else { (ok, fail + 1) }
        })
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("mirror-gallery -- clone and fetch all repos for GitHub owners");
        eprintln!();
        eprintln!("USAGE: mirror-gallery <owner> [<owner> ...]");
        eprintln!();
        eprintln!("Repos land in $MIRROR_GALLERY_ROOT/<owner>/<repo>");
        eprintln!("Default root: ~/Mirrors/Github");
        eprintln!();
        eprintln!("Private repos are included when `gh` has access.");
        eprintln!("Authenticate once with: gh auth login");
        return ExitCode::from(u8::from(args.is_empty()));
    }

    // ── Pre-flight: verify gh authentication ──
    eprintln!("mirror-gallery: checking GitHub authentication...");
    match preflight_gh_auth() {
        Ok(user) => {
            eprintln!("mirror-gallery: authenticated as {user}");
            eprintln!("mirror-gallery: private repos accessible to {user} will be included");
        }
        Err(detail) => {
            eprintln!();
            eprintln!("========================================");
            eprintln!("  mirror-gallery: NOT AUTHENTICATED");
            eprintln!("========================================");
            eprintln!();
            eprintln!("The `gh` CLI is not logged in to github.com.");
            eprintln!("Without authentication:");
            eprintln!("  - only PUBLIC repositories can be listed");
            eprintln!("  - private repositories will be invisible");
            eprintln!("  - org repos you have access to may be missing");
            eprintln!();
            eprintln!("To fix this, run ONCE on this machine:");
            eprintln!();
            eprintln!("    gh auth login");
            eprintln!();
            eprintln!("Then re-run mirror-gallery.");
            eprintln!();
            eprintln!("gh said: {}", detail.trim());
            eprintln!("========================================");
            return ExitCode::FAILURE;
        }
    }

    let root = resolve_root();
    eprintln!("mirror-gallery: root = {}", root.display());

    if let Err(e) = fs::create_dir_all(&root) {
        eprintln!("mirror-gallery: cannot create root: {e}");
        return ExitCode::FAILURE;
    }

    let (total_ok, total_fail) = args
        .iter()
        .map(|owner| mirror_owner(&root, owner))
        .fold((0usize, 0usize), |(a, b), (c, d)| (a + c, b + d));

    eprintln!();
    eprintln!("mirror-gallery: {total_ok} ok, {total_fail} failed");

    if total_fail > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
