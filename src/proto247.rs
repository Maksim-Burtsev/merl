use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI8, Ordering};

use anyhow::{Context, Result, bail};

#[derive(Clone)]
struct Item {
    path: String,
    branch: String,
    done: bool,
    gone: bool,
}

impl Item {
    fn name(&self) -> String {
        format!("{} \u{b7} {}", self.path, self.branch)
    }
}

struct Batch {
    folder: PathBuf,
    items: Vec<Item>,
    at: usize,
    rest: Vec<OsString>,
}

static BATCH: Mutex<Option<Batch>> = Mutex::new(None);
static SWITCH: Mutex<Option<usize>> = Mutex::new(None);
static ARRIVAL: Mutex<Option<String>> = Mutex::new(None);
static END: AtomicI8 = AtomicI8::new(0);
static PREV: AtomicI8 = AtomicI8::new(0);

const FILE: &str = ".merl-batch";

fn env_is(name: &str, value: &str) -> bool {
    std::env::var(name).is_ok_and(|m| m == value)
}

pub fn starts_on_list() -> bool {
    env_is("MERL_PROTO_247_START", "list") && on()
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
                gone: false,
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
    let word = |n: usize| args.get(n).and_then(|a| a.to_str());
    match (word(1), word(2)) {
        (Some("--new-batch"), _) => {
            create()?;
            std::process::exit(0)
        }
        (Some("--batch"), Some("--list")) => {
            list()?;
            std::process::exit(0)
        }
        (Some("--batch"), _) => open(args.iter().skip(2).cloned().collect()),
        _ => Ok(args),
    }
}

fn git_ok(repo: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .is_ok_and(|o| o.status.success())
}

fn on_origin(repo: &Path, branch: &str) -> bool {
    let name = format!("refs/remotes/origin/{branch}");
    git_ok(repo, &["rev-parse", "-q", "--verify", &name])
}

fn s(n: usize) -> &'static str {
    if n == 1 { "" } else { "es" }
}

fn create() -> Result<()> {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text)?;
    let here = std::env::current_dir()?;
    let file = here.join(FILE);
    let old = read(&file).unwrap_or_default();
    let mut fresh: Vec<Item> = Vec::new();
    let (mut kept, mut new) = (0, 0);
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (path, branch) = line
            .rsplit_once(':')
            .with_context(|| format!("{line}: PATH:BRANCH"))?;
        let repo = here.join(path);
        if !(git_ok(&repo, &["fetch", "-q", "--prune", "origin"]) && on_origin(&repo, branch)) {
            bail!("{path}:{branch}: no such branch on origin; nothing saved");
        }
        if fresh.iter().any(|f| f.path == path && f.branch == branch) {
            continue;
        }
        let before = old.iter().find(|o| o.path == path && o.branch == branch);
        match before {
            Some(o) if !o.done => kept += 1,
            Some(_) => {}
            None => new += 1,
        }
        fresh.push(Item {
            path: path.to_owned(),
            branch: branch.to_owned(),
            done: before.is_some_and(|o| o.done),
            gone: false,
        });
    }
    if fresh.is_empty() {
        if file.exists() {
            std::fs::remove_file(&file)?;
        }
        println!("batch removed");
        return Ok(());
    }
    write(&file, &fresh)?;
    let repos: BTreeSet<&str> = fresh.iter().map(|i| i.path.as_str()).collect();
    let (n, m) = (fresh.len(), repos.len());
    let ies = if m == 1 { "y" } else { "ies" };
    let from = if old.is_empty() {
        String::new()
    } else {
        format!(" ({kept} unread from before, {new} new)")
    };
    println!("batch: {n} branch{} in {m} repositor{ies}{from}", s(n));
    println!("cd {} && merl --batch", here.display());
    Ok(())
}

fn load(folder: &Path) -> Result<Vec<Item>> {
    let file = folder.join(FILE);
    if !file.exists() {
        bail!("no batch in this folder");
    }
    let mut items = read(&file)?;
    if items.is_empty() {
        bail!("no batch in this folder");
    }
    let repos: BTreeSet<String> = items.iter().map(|i| i.path.clone()).collect();
    for repo in repos {
        git_ok(&folder.join(repo), &["fetch", "-q", "--prune", "origin"]);
    }
    for i in &mut items {
        i.gone = !on_origin(&folder.join(&i.path), &i.branch);
    }
    Ok(items)
}

fn state(i: &Item) -> &'static str {
    match (i.gone, i.done) {
        (true, _) => "gone",
        (false, true) => "read",
        (false, false) => "unread",
    }
}

fn list() -> Result<()> {
    for i in load(&std::env::current_dir()?)? {
        println!("{:<6}  {}:{}", state(&i), i.path, i.branch);
    }
    Ok(())
}

fn open(rest: Vec<OsString>) -> Result<Vec<OsString>> {
    let folder = std::env::current_dir()?;
    let mut items = load(&folder)?;
    let mut note = String::new();
    if env_is("MERL_PROTO_247_GONE", "leave") {
        let left: Vec<String> = items.iter().filter(|i| i.gone).map(Item::name).collect();
        if !left.is_empty() {
            items.retain(|i| !i.gone);
            write(&folder.join(FILE), &items)?;
            note = format!("{} left the batch: gone from origin", left.join(", "));
        }
    }
    let n = items.len();
    let read = items.iter().filter(|i| i.done).count();
    let Some(at) = items.iter().position(|i| !i.done && !i.gone) else {
        println!(
            "merl: nothing left to read: all {n} branch{} of the batch are read",
            s(n)
        );
        std::process::exit(0)
    };
    if note.is_empty() && read > 0 {
        note = format!("{read} of {n} read");
    }
    if !note.is_empty() {
        *lock(&ARRIVAL) = Some(note);
    }
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

fn says_less() -> bool {
    env_is("MERL_PROTO_247_SAYS", "less")
}

pub fn arrival() -> Option<String> {
    lock(&ARRIVAL).take().filter(|_| !says_less())
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

fn neighbour(items: &[Item], at: usize, dir: isize) -> Option<(usize, Vec<String>)> {
    let mut passed = Vec::new();
    let mut i = at;
    loop {
        i = i.checked_add_signed(dir)?;
        let item = items.get(i)?;
        if !item.gone {
            return Some((i, passed));
        }
        passed.push(item.name());
    }
}

fn cross(b: &Batch, to: usize, passed: &[String]) {
    let name = b.items.get(to).map(Item::name).unwrap_or_default();
    let mut text = name;
    if !passed.is_empty() {
        write!(text, " (passed {}: gone from origin)", passed.join(", ")).ok();
    }
    *lock(&ARRIVAL) = Some(text);
    *lock(&SWITCH) = Some(to);
}

pub fn at_end(dir: isize) -> String {
    let mut g = lock(&BATCH);
    let Some(b) = g.as_mut() else {
        return String::new();
    };
    let at = b.at;
    if dir > 0 {
        if let Some(i) = b.items.get_mut(at) {
            i.done = true;
        }
        drop(write(&b.folder.join(FILE), &b.items));
    }
    let sign: i8 = if dir > 0 { 1 } else { -1 };
    let next = neighbour(&b.items, at, dir);
    let (key, edge) = if dir > 0 {
        ("c", "last")
    } else {
        ("C", "first")
    };
    match next {
        Some((to, passed)) if PREV.load(Ordering::Relaxed) == sign => {
            cross(b, to, &passed);
            String::new()
        }
        Some(_) if says_less() => {
            END.store(sign, Ordering::Relaxed);
            format!("{edge} hunk of the branch")
        }
        Some((to, _)) => {
            END.store(sign, Ordering::Relaxed);
            let name = b.items.get(to).map(Item::name).unwrap_or_default();
            format!("{edge} hunk of the branch, {key} opens {name}")
        }
        None if says_less() => format!("{edge} hunk of the batch"),
        None if dir > 0 => {
            let n = b.items.iter().filter(|i| !i.gone).count();
            let unread = b.items.iter().filter(|i| !i.gone && !i.done).count();
            if unread == 0 {
                format!("last hunk of the batch, all {n} read")
            } else {
                format!("last hunk of the batch, {unread} of {n} unread")
            }
        }
        None => "first hunk of the batch".into(),
    }
}

pub fn picker_labels() -> Option<(String, Vec<String>, usize)> {
    let g = lock(&BATCH);
    let b = g.as_ref()?;
    let w = b.items.iter().map(|i| i.path.len()).max().unwrap_or(0);
    let bw = b.items.iter().map(|i| i.branch.len()).max().unwrap_or(0);
    let labels = b
        .items
        .iter()
        .enumerate()
        .map(|(n, i)| {
            let tick = if i.done && !i.gone { "\u{2713}" } else { " " };
            let gone = if i.gone { "  gone from origin" } else { "" };
            format!(
                "{tick} {:>2}  {:<w$}  {:<bw$}{gone}",
                n + 1,
                i.path,
                i.branch
            )
        })
        .collect();
    let n = b.items.len();
    let read = b.items.iter().filter(|i| i.done && !i.gone).count();
    Some((format!("Batch: {read} of {n} read"), labels, b.at))
}

pub fn pick(index: usize) -> Option<String> {
    let g = lock(&BATCH);
    let b = g.as_ref()?;
    let item = b.items.get(index)?;
    if item.gone {
        return Some(format!("{} is gone from origin", item.name()));
    }
    if b.at != index {
        *lock(&ARRIVAL) = Some(item.name());
        *lock(&SWITCH) = Some(index);
    }
    None
}

pub fn farewell() -> Option<String> {
    if says_less() {
        return None;
    }
    let g = lock(&BATCH);
    let b = g.as_ref()?;
    let n = b.items.iter().filter(|i| !i.gone).count();
    let read = b.items.iter().filter(|i| i.done && !i.gone).count();
    Some(match b.items.iter().find(|i| !i.done && !i.gone) {
        Some(i) => format!(
            "{read} of {n} branches read; merl --batch goes on at {}",
            i.name()
        ),
        None => format!("all {n} branches of the batch are read"),
    })
}
