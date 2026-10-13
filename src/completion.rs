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
        .filter(|n| !n.ends_with("/HEAD"))
        .collect();
    names.sort();
    names.dedup();
    let prefix: Vec<String> = names
        .iter()
        .filter(|n| n.starts_with(current.as_ref()))
        .cloned()
        .collect();
    let picked = if !prefix.is_empty() || std::env::var_os("MERL_PROTO_PREFIX").is_some() {
        prefix
    } else {
        names
            .into_iter()
            .filter(|n| n.contains(current.as_ref()))
            .collect()
    };
    picked.into_iter().map(CompletionCandidate::new).collect()
}

pub fn zsh_script(completer: &str) -> String {
    use clap_complete::env::EnvCompleter;
    let mut buf = Vec::new();
    clap_complete::env::Zsh
        .write_registration("COMPLETE", "merl", "merl", completer, &mut buf)
        .unwrap();
    let script = String::from_utf8(buf).unwrap();
    let script = if std::env::var_os("MERL_PROTO_ZSH_PLAIN").is_some() {
        script
    } else {
        script.replace(
            "_describe -V 'values' other\n",
            "_describe -V 'values' other -M 'l:|=* r:|=*'\n",
        )
    };
    format!("{script}\n[[ $funcstack[1] == _merl ]] && _clap_dynamic_completer_merl \"$@\"\n")
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

pub fn bash_script(completer: &str) -> String {
    use clap_complete::env::EnvCompleter;
    let mut buf = Vec::new();
    clap_complete::env::Bash
        .write_registration("COMPLETE", "merl", "merl", completer, &mut buf)
        .unwrap();
    let script = String::from_utf8(buf).unwrap();
    if std::env::var_os("MERL_PROTO_BASH_PLAIN").is_some() {
        return script;
    }
    script.replacen(
        "    elif [[ $_CLAP_COMPLETE_SPACE == false ]]",
        "    elif [[ ${#COMPREPLY[@]} -gt 1 && ${COMPREPLY[0]} != \"$2\"* ]]; then\n        COMPREPLY+=(\"$2\")\n    elif [[ $_CLAP_COMPLETE_SPACE == false ]]",
        1,
    )
}
