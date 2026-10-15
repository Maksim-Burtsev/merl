use super::*;

#[test]
fn epic_body_closes_what_the_ledger_does_not_mark_part() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = std::env::temp_dir().join(format!("merl-epic-body-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    let gh = dir.join("bin/gh");
    std::fs::write(&gh, "#!/bin/sh\necho \"$@\"\n").unwrap();
    std::fs::set_permissions(&gh, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let ledger = "| Lane | Task | Kind | Branch | Agent | State |\n|---|---|---|---|---|---|\n\
        | A | #1 cmake | epic | b/1 | x | merged in epic; upstream report left to owner |\n\
        | B | #2 #3 ts | epic | b/2 | x | part #3: merged in epic |\n\
        | C | #4 docs | epic | b/4 | x | PART: merged in epic |\n";
    std::fs::write(dir.join("ledger.md"), ledger).unwrap();
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
