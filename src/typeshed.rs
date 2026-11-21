use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const COMMIT: &str = "e4a2f2272550112da39dbbad67b0b6ca778586db";

static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

fn location() -> Option<PathBuf> {
    Some(
        dirs::home_dir()?
            .join(".cache/merl/typeshed")
            .join(COMMIT)
            .join("stdlib"),
    )
}

pub fn stdlib() -> Option<&'static Path> {
    DIR.get_or_init(|| unpacked(&location()?)).as_deref()
}

pub fn within(path: &Path) -> Option<&Path> {
    path.strip_prefix(location()?).ok()
}

#[cfg(target_family = "wasm")]
fn unpacked(_: &Path) -> Option<PathBuf> {
    None
}

#[cfg(not(target_family = "wasm"))]
fn unpacked(dir: &Path) -> Option<PathBuf> {
    if dir.is_dir() {
        return Some(dir.to_path_buf());
    }
    let part = dir.with_extension(std::process::id().to_string());
    let written = write_stubs(&part);
    if written.is_err() || std::fs::rename(&part, dir).is_err() {
        drop(std::fs::remove_dir_all(&part));
    }
    dir.is_dir().then(|| dir.to_path_buf())
}

#[cfg(not(target_family = "wasm"))]
fn write_stubs(dir: &Path) -> std::io::Result<()> {
    let mut text = String::new();
    std::io::Read::read_to_string(
        &mut flate2::read::ZlibDecoder::new(&include_bytes!("../typeshed/stdlib.zlib")[..]),
        &mut text,
    )?;
    let mut fields = text.split('\0');
    while let (Some(name), Some(stub)) = (fields.next(), fields.next()) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, stub)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn a_stub_is_within_typeshed_before_anything_in_the_process_unpacks_it() {
        const CHILD: &str = "MERL_TEST_TYPESHED_WITHIN";
        if let Some(home) = std::env::var_os(CHILD) {
            let stub = Path::new(&home)
                .join(".cache/merl/typeshed")
                .join(super::COMMIT)
                .join("stdlib/json/__init__.pyi");
            println!("within={:?}", super::within(&stub));
            return;
        }
        let home = std::env::temp_dir().join(format!("merl-within-home-{}", std::process::id()));
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "typeshed::tests::a_stub_is_within_typeshed_before_anything_in_the_process_unpacks_it",
                "--nocapture",
            ])
            .env(CHILD, &home)
            .env("HOME", &home)
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&out.stdout);
        let failed = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{said}{failed}");
        assert!(
            said.contains(r#"within=Some("json/__init__.pyi")"#),
            "{said}"
        );
    }
}
