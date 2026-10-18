use std::io;
#[cfg(target_family = "wasm")]
use std::path::Component;
use std::path::{Path, PathBuf};

#[cfg(not(target_family = "wasm"))]
pub fn canonicalize(p: &Path) -> io::Result<PathBuf> {
    p.canonicalize()
}

#[cfg(target_family = "wasm")]
pub fn canonicalize(p: &Path) -> io::Result<PathBuf> {
    std::fs::metadata(p)?;
    let mut out = PathBuf::from("/");
    for c in p.components() {
        match c {
            Component::Normal(n) => out.push(n),
            Component::ParentDir => {
                out.pop();
            }
            _ => {}
        }
    }
    Ok(out)
}

#[cfg(not(target_family = "wasm"))]
pub fn pid() -> u32 {
    std::process::id()
}

#[cfg(target_family = "wasm")]
pub fn pid() -> u32 {
    1
}
