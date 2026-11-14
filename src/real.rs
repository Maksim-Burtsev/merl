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

pub fn link_inside(root: &Path, rel: &Path) -> Option<PathBuf> {
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
    let Ok(target) = path.canonicalize() else {
        return std::fs::write(path, bytes);
    };
    let old = (std::fs::OpenOptions::new().write(true).open(&target))?.metadata()?;
    let name = target.file_name().unwrap_or_default().to_string_lossy();
    let tmp = target.with_file_name(format!(".{name}.merl-{}", pid()));
    let file = match std::fs::File::create(&tmp) {
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            return std::fs::write(&target, bytes);
        }
        made => made?,
    };
    let replaced = replace(file, &old, &tmp, &target, bytes);
    if !matches!(replaced, Ok(true)) {
        drop(std::fs::remove_file(&tmp));
    }
    if replaced? {
        Ok(())
    } else {
        std::fs::write(&target, bytes)
    }
}

#[cfg(not(target_family = "wasm"))]
fn replace(
    mut file: std::fs::File,
    old: &std::fs::Metadata,
    tmp: &Path,
    target: &Path,
    bytes: &[u8],
) -> io::Result<bool> {
    use std::io::Write;
    use std::os::unix::fs::MetadataExt;
    let new = file.metadata()?;
    if old.nlink() > 1 || (old.uid(), old.gid()) != (new.uid(), new.gid()) {
        return Ok(false);
    }
    file.set_permissions(old.permissions())?;
    file.write_all(bytes)?;
    file.sync_all()?;
    std::fs::rename(tmp, target)?;
    Ok(true)
}

#[cfg(target_family = "wasm")]
pub fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    std::fs::write(path, bytes)
}
