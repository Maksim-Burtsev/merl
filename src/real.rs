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

#[cfg(test)]
thread_local! {
    pub static LINK_CHECKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub fn link_inside(root: &Path, rel: &Path) -> Option<PathBuf> {
    #[cfg(test)]
    LINK_CHECKS.with(|c| c.set(c.get() + 1));
    let path = root.join(rel);
    if !path.is_symlink() {
        return None;
    }
    let real = canonicalize(&path).ok()?;
    let real = real.strip_prefix(canonicalize(root).ok()?).ok()?;
    (real != rel).then(|| real.to_path_buf())
}

#[cfg(not(target_family = "wasm"))]
pub fn pid() -> u32 {
    std::process::id()
}

#[cfg(target_family = "wasm")]
pub fn pid() -> u32 {
    1
}

#[cfg(not(target_family = "wasm"))]
pub fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_after_probe(path, bytes, io::Write::write_all)
}

#[cfg(not(target_family = "wasm"))]
pub fn write_after_probe(
    path: &Path,
    bytes: &[u8],
    write: impl FnOnce(&mut std::fs::File, &[u8]) -> io::Result<()>,
) -> io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    if let Ok(target) = path.canonicalize() {
        let name = target.file_name().unwrap_or_default().to_string_lossy();
        let tmp = target.with_file_name(format!(".{name}.merl-{}", pid()));
        let made = (std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600))
        .open(&tmp);
        if let Ok(mut file) = made {
            let probed = write(&mut file, bytes).and_then(|()| file.sync_all());
            drop(file);
            drop(std::fs::remove_file(&tmp));
            probed?;
        }
    }
    std::fs::write(path, bytes)
}

#[cfg(target_family = "wasm")]
pub fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    std::fs::write(path, bytes)
}
