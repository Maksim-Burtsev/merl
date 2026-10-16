use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Instant, SystemTime};

use anyhow::{Context, Result, bail};

pub fn variant() -> char {
    std::env::var("MERL_925")
        .ok()
        .and_then(|v| v.chars().next())
        .unwrap_or('A')
}

fn dir() -> Result<PathBuf> {
    let user = std::env::var("USER").unwrap_or_else(|_| "merl".into());
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(|| PathBuf::from(format!("/tmp/merl-{user}")), |d| PathBuf::from(d).join("merl"));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .recursive(true)
        .create(&dir)?;
    let meta = std::fs::metadata(&dir)?;
    let home = std::env::var_os("HOME").context("no home")?;
    let mine = std::fs::metadata(home)?.uid() == meta.uid();
    if !mine || meta.mode() & 0o077 != 0 {
        bail!("{} is not private", dir.display());
    }
    Ok(dir)
}

pub struct Listening {
    sock: PathBuf,
    root_file: PathBuf,
    touched: Instant,
}

impl Drop for Listening {
    fn drop(&mut self) {
        drop(std::fs::remove_file(&self.sock));
        drop(std::fs::remove_file(&self.root_file));
    }
}

impl Listening {
    pub fn pressed(&mut self) {
        if self.touched.elapsed().as_millis() < 500 {
            return;
        }
        self.touched = Instant::now();
        if let Ok(f) = std::fs::File::options().write(true).open(&self.root_file) {
            drop(f.set_modified(SystemTime::now()));
        }
    }
}

pub fn listen<M: Send + 'static>(
    root: &Path,
    name: Option<&str>,
    tx: Sender<M>,
    msg: fn(PathBuf, usize, usize) -> M,
) -> Option<Listening> {
    let dir = dir().ok()?;
    let name = name.map_or_else(|| std::process::id().to_string(), str::to_owned);
    let sock = dir.join(format!("{name}.sock"));
    if UnixStream::connect(&sock).is_err() {
        drop(std::fs::remove_file(&sock));
    }
    let listener = UnixListener::bind(&sock).ok()?;
    drop(std::fs::set_permissions(
        &sock,
        std::fs::Permissions::from_mode(0o600),
    ));
    let root_file = dir.join(format!("{name}.root"));
    std::fs::write(&root_file, root.as_os_str().as_encoded_bytes()).ok()?;
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();
            if BufReader::new(&stream).read_line(&mut line).is_err() {
                continue;
            }
            let mut parts = line.trim_end().split('\t');
            let (Some(path), Some(l), Some(c)) = (parts.next(), parts.next(), parts.next()) else {
                continue;
            };
            let (Ok(l), Ok(c)) = (l.parse(), c.parse()) else {
                continue;
            };
            if tx.send(msg(PathBuf::from(path), l, c)).is_err() {
                break;
            }
        }
    });
    Some(Listening {
        sock,
        root_file,
        touched: Instant::now(),
    })
}

struct Open {
    name: String,
    root: PathBuf,
    pressed: SystemTime,
}

fn open_merls() -> Vec<Open> {
    let Ok(dir) = dir() else { return vec![] };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![];
    };
    entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.extension()? != "root" {
                return None;
            }
            let name = p.file_stem()?.to_str()?.to_owned();
            if UnixStream::connect(dir.join(format!("{name}.sock"))).is_err() {
                return None;
            }
            Some(Open {
                root: PathBuf::from(std::fs::read_to_string(&p).ok()?),
                pressed: e.metadata().ok()?.modified().ok()?,
                name,
            })
        })
        .collect()
}

pub fn receiver(file: &Path, server: Option<&str>) -> Option<String> {
    let mut found: Vec<Open> = open_merls()
        .into_iter()
        .filter(|o| file.starts_with(&o.root))
        .filter(|o| server.is_none_or(|s| s == o.name))
        .collect();
    found.sort_by_key(|o| o.pressed);
    found.pop().map(|o| o.name)
}

pub fn send(name: &str, file: &Path, line: usize, col: usize) -> Result<()> {
    let mut s = UnixStream::connect(dir()?.join(format!("{name}.sock")))
        .with_context(|| format!("no merl named {name}"))?;
    writeln!(s, "{}\t{line}\t{col}", file.display())?;
    Ok(())
}
