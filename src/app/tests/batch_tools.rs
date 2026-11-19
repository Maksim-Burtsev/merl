use super::*;

const COMMON: &str =
    "- PR bodies end with:\n    Footer line\n\n- Commit trailers:\n    Trailer: x\n";

fn epic_body(name: &str, ledger: &str, common: &str) -> (std::process::Output, PathBuf) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = std::env::temp_dir().join(format!("merl-{name}-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    let gh = dir.join("bin/gh");
    std::fs::write(&gh, "#!/bin/sh\necho \"$@\"\n").unwrap();
    std::fs::set_permissions(&gh, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    std::fs::write(dir.join("ledger.md"), ledger).unwrap();
    std::fs::write(dir.join("COMMON.md"), common).unwrap();
    let path = format!(
        "{}:{}",
        dir.join("bin").display(),
        std::env::var("PATH").unwrap()
    );
    let run = std::process::Command::new("python3")
        .arg(root.join(".claude/skills/batch/bin/epic-body"))
        .arg(&dir)
        .env("PATH", path)
        .env("HOME", &dir)
        .output()
        .unwrap();
    (run, dir)
}

#[test]
fn epic_body_closes_what_the_ledger_does_not_mark_part() {
    let (run, dir) = epic_body(
        "epic-body",
        "| Lane | Task | Kind | Branch | Agent | State |\n|---|---|---|---|---|---|\n\
        | A | #1 cmake | epic | b/1 | x | merged in epic; upstream report left to owner |\n\
        | B | #2 #3 ts | epic | b/2 | x | part #3: merged in epic |\n\
        | C | #4 docs | epic | b/4 | x | PART: merged in epic |\n",
        COMMON,
    );
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let body = std::fs::read_to_string(dir.join("epic-body.md")).unwrap();
    assert!(
        body.starts_with("Closes #1, closes #2.\n\nPart of #3, #4:"),
        "{body}"
    );
    assert!(
        body.contains("### #1: issue view 1 --repo Maksim-Burtsev/merl "),
        "{body}"
    );
}

#[test]
fn epic_body_names_the_header_it_reads_and_writes_nothing_without_it() {
    for ledger in [
        "| Task | Issue | Kind | Branch | Agent | State |\n|---|---|---|---|---|---|\n\
         | str1530 | #1530 | epic | b/str1530 | x | merged into epic |\n",
        "| Name | Task | Kind | Branch | Agent | State |\n|---|---|---|---|---|---|\n\
         | str1530 | #1530 | own PR | b/str1530 | x | PR open |\n",
        "no table\n",
    ] {
        let (run, dir) = epic_body("epic-body-header", ledger, "");
        assert!(!run.status.success(), "{ledger}");
        assert!(
            String::from_utf8_lossy(&run.stderr)
                .contains("| Name | Task | Kind | Branch | Agent | State | Tokens | Min |"),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert!(!dir.join("epic-body.md").exists(), "{ledger}");
    }
}

#[test]
fn epic_body_names_the_footer_label_it_reads_and_writes_nothing_without_it() {
    let ledger = "| Name | Task | Kind | Branch | Agent | State | Tokens | Min |\n|---|---|---|---|---|---|---|---|\n\
        | str1530 | #1530 | epic | b/str1530 | x | merged into epic | 1 | 1 |\n";
    let (run, dir) = epic_body(
        "epic-body-footer",
        ledger,
        &COMMON.replace(
            "PR bodies end with:",
            "PR bodies (only if a task file asks for a PR) end with:",
        ),
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(!run.status.success(), "{stderr}");
    assert!(stderr.contains("under PR bodies end with: in "), "{stderr}");
    assert!(!dir.join("epic-body.md").exists(), "{stderr}");
    let (run, dir) = epic_body(
        "epic-body-trailers",
        ledger,
        "- PR bodies end with:\n    Footer line\n",
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(!run.status.success(), "{stderr}");
    assert!(stderr.contains("under Commit trailers: in "), "{stderr}");
    assert!(!dir.join("epic-body.md").exists(), "{stderr}");
    let (run, dir) = epic_body("epic-body-footer", ledger, COMMON);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let body = std::fs::read_to_string(dir.join("epic-body.md")).unwrap();
    assert!(body.ends_with("Keys — none\n\nFooter line\n"), "{body}");
    let squash = std::fs::read_to_string(dir.join("squash-msg.txt")).unwrap();
    assert!(squash.ends_with("\n\nTrailer: x\n"), "{squash}");
}
