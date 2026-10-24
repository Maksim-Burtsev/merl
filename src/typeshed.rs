use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const COMMIT: &str = "e4a2f2272550112da39dbbad67b0b6ca778586db";

static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

pub fn stdlib() -> Option<&'static Path> {
    DIR.get_or_init(|| unpacked(&dirs::home_dir()?.join(".cache/merl/typeshed").join(COMMIT)))
        .as_deref()
}

pub fn within(path: &Path) -> Option<&Path> {
    path.strip_prefix(DIR.get()?.as_deref()?).ok()
}

#[cfg(target_family = "wasm")]
fn unpacked(_: &Path) -> Option<PathBuf> {
    None
}

#[cfg(not(target_family = "wasm"))]
fn unpacked(at: &Path) -> Option<PathBuf> {
    let dir = at.join("stdlib");
    if dir.is_dir() {
        return Some(dir);
    }
    let part = at.join(format!("stdlib.{}", std::process::id()));
    let written = write_stubs(&part);
    if written.is_err() || std::fs::rename(&part, &dir).is_err() {
        drop(std::fs::remove_dir_all(&part));
    }
    dir.is_dir().then_some(dir)
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
