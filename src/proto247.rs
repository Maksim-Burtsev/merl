use std::ffi::OsString;
use std::fmt::Write as _;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI8, Ordering};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, bail};

#[derive(Clone)]
struct Item {
    path: String,
    branch: String,
    done: bool,
}

struct Batch {
    folder: PathBuf,
    items: Vec<Item>,
    at: usize,
    rest: Vec<OsString>,
}

static BATCH: Mutex<Option<Batch>> = Mutex::new(None);
static SWITCH: Mutex<Option<usize>> = Mutex::new(None);
static END: AtomicI8 = AtomicI8::new(0);
static PREV: AtomicI8 = AtomicI8::new(0);

const FILE: &str = ".merl-batch";

fn replace_mode() -> bool {
    std::env::var("MERL_PROTO_247").is_ok_and(|m| m == "replace")
}

fn expire_mode() -> bool {
    std::env::var("MERL_PROTO_247_EXPIRE").is_ok_and(|m| m == "14")
}

fn read(file: &Path) -> Result<Vec<Item>> {
    Ok(std::fs::read_to_string(file)?
        .lines()
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some(Item {
                path: f.next()?.to_owned(),
                branch: f.next()?.to_owned(),
                done: f.next() == Some("done"),
            })
        })
        .collect())
}

fn write(file: &Path, items: &[Item]) -> Result<()> {
    let mut out = String::new();
    for i in items {
        let state = if i.done { "done" } else { "left" };
        writeln!(out, "{}\t{}\t{state}", i.path, i.branch)?;
    }
    std::fs::write(file, out)?;
    Ok(())
}

pub fn args() -> Result<Vec<OsString>> {
    let args: Vec<OsString> = std::env::args_os().collect();
    match args.get(1).and_then(|a| a.to_str()) {
        Some("--new-batch") => {
            create()?;
            std::process::exit(0)
        }
        Some("--batch") => open(args.iter().skip(2).cloned().collect()),
        _ => Ok(args),
    }
}

fn create() -> Result<()> {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text)?;
    let here = std::env::current_dir()?;
    let mut fresh = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (path, branch) = line
            .rsplit_once(':')
            .with_context(|| format!("{line}: PATH:BRANCH"))?;
        let repo = here.join(path);
        let ok = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["fetch", "-q", "origin"])
            .status()
            .is_ok_and(|s| s.success())
            && Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args(["rev-parse", "-q", "--verify"])
                .arg(format!("refs/remotes/origin/{branch}"))
                .output()
                .is_ok_and(|o| o.status.success());
        if !ok {
            bail!("{path}:{branch}: no such branch on origin; nothing saved");
        }
        fresh.push(Item {
            path: path.to_owned(),
            branch: branch.to_owned(),
            done: false,
        });
    }
    let file = here.join(FILE);
    let old = read(&file).unwrap_or_default();
    let left: Vec<Item> = if replace_mode() {
        Vec::new()
    } else {
        old.into_iter().filter(|i| !i.done).collect()
    };
    let same = |a: &Item, b: &Item| a.path == b.path && a.branch == b.branch;
    let added: Vec<Item> = fresh
        .into_iter()
        .filter(|f| !left.iter().any(|l| same(l, f)))
        .collect();
    let (l, n) = (left.len(), added.len());
    let all: Vec<Item> = left.into_iter().chain(added).collect();
    write(&file, &all)?;
    if l == 0 {
        println!("merl --batch");
    } else {
        println!("merl --batch   ({l} left from before, {n} new)");
    }
    Ok(())
}

fn open(rest: Vec<OsString>) -> Result<Vec<OsString>> {
    let folder = std::env::current_dir()?;
    let file = folder.join(FILE);
    if !file.exists() {
        bail!("no batch in this folder");
    }
    let age = std::fs::metadata(&file)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .unwrap_or_default();
    if expire_mode() && age > Duration::from_secs(14 * 86_400) {
        std::fs::remove_file(&file)?;
        bail!("no batch in this folder");
    }
    let items = read(&file)?;
    if items.is_empty() {
        bail!("no batch in this folder");
    }
    std::fs::File::options()
        .append(true)
        .open(&file)?
        .set_modified(SystemTime::now())?;
    let at = items.iter().position(|i| !i.done).unwrap_or(0);
    *lock(&BATCH) = Some(Batch {
        folder,
        items,
        at,
        rest,
    });
    item_args(at).context("an empty batch")
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn item_args(at: usize) -> Option<Vec<OsString>> {
    let mut g = lock(&BATCH);
    let b = g.as_mut()?;
    let item = b.items.get(at)?.clone();
    b.at = at;
    std::env::set_current_dir(b.folder.join(&item.path)).ok()?;
    let mut out: Vec<OsString> = std::env::args_os().take(1).collect();
    out.push(format!("--review={}", item.branch).into());
    out.extend(b.rest.iter().cloned());
    Some(out)
}

pub fn next_args() -> Option<Vec<OsString>> {
    let at = lock(&SWITCH).take()?;
    END.store(0, Ordering::Relaxed);
    PREV.store(0, Ordering::Relaxed);
    item_args(at)
}

pub fn switch_pending() -> bool {
    lock(&SWITCH).is_some()
}

pub fn on() -> bool {
    lock(&BATCH).is_some()
}

pub fn repo_label() -> Option<String> {
    let g = lock(&BATCH);
    let b = g.as_ref()?;
    Some(b.items.get(b.at)?.path.clone())
}

pub fn branch_label() -> Option<String> {
    let g = lock(&BATCH);
    let b = g.as_ref()?;
    Some(format!("branch {}/{}", b.at + 1, b.items.len()))
}

pub fn begin_key() {
    PREV.store(END.swap(0, Ordering::Relaxed), Ordering::Relaxed);
}

pub fn at_end(dir: isize) -> String {
    let mut g = lock(&BATCH);
    let Some(b) = g.as_mut() else {
        return String::new();
    };
    let (at, n) = (b.at, b.items.len());
    if dir > 0 {
        if let Some(i) = b.items.get_mut(at) {
            i.done = true;
        }
        drop(write(&b.folder.join(FILE), &b.items));
        if PREV.load(Ordering::Relaxed) == 1 && at + 1 < n {
            *lock(&SWITCH) = Some(at + 1);
            return String::new();
        }
        END.store(1, Ordering::Relaxed);
        if at + 1 == n {
            "last hunk of the batch".into()
        } else {
            "last hunk of the branch".into()
        }
    } else {
        if PREV.load(Ordering::Relaxed) == -1 && at > 0 {
            *lock(&SWITCH) = Some(at - 1);
            return String::new();
        }
        END.store(-1, Ordering::Relaxed);
        if at == 0 {
            "first hunk of the batch".into()
        } else {
            "first hunk of the branch".into()
        }
    }
}

pub fn picker_labels() -> Option<(Vec<String>, usize)> {
    let g = lock(&BATCH);
    let b = g.as_ref()?;
    let w = b.items.iter().map(|i| i.path.len()).max().unwrap_or(0);
    let labels = b
        .items
        .iter()
        .enumerate()
        .map(|(n, i)| {
            let tick = if i.done { "\u{2713}" } else { " " };
            format!("{tick} {:>2}  {:<w$}  {}", n + 1, i.path, i.branch)
        })
        .collect();
    Some((labels, b.at))
}

pub fn pick(index: usize) {
    let current = lock(&BATCH).as_ref().map(|b| b.at);
    if current != Some(index) {
        *lock(&SWITCH) = Some(index);
    }
}
