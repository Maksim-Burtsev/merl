use std::ffi::OsStr;
use std::process::Command;

use clap_complete::engine::CompletionCandidate;

fn refs(patterns: &[&str]) -> Vec<String> {
    let out = Command::new("git")
        .args(["for-each-ref", "--format=%(refname)"])
        .args(patterns)
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn matching(names: impl IntoIterator<Item = String>, current: &OsStr) -> Vec<CompletionCandidate> {
    let current = current.to_string_lossy();
    let mut names: Vec<String> = names
        .into_iter()
        .filter(|n| n.starts_with(current.as_ref()) && !n.ends_with("/HEAD"))
        .collect();
    names.sort();
    names.dedup();
    names.into_iter().map(CompletionCandidate::new).collect()
}

pub fn review(current: &OsStr) -> Vec<CompletionCandidate> {
    let names = refs(&["refs/heads", "refs/remotes/origin"])
        .into_iter()
        .filter_map(|r| {
            r.strip_prefix("refs/heads/")
                .or_else(|| r.strip_prefix("refs/remotes/origin/"))
                .map(str::to_string)
        });
    matching(names, current)
}

pub fn base(current: &OsStr) -> Vec<CompletionCandidate> {
    let names = refs(&["refs/heads", "refs/remotes"])
        .into_iter()
        .filter_map(|r| {
            r.strip_prefix("refs/heads/")
                .or_else(|| r.strip_prefix("refs/remotes/"))
                .map(str::to_string)
        });
    matching(names, current)
}
