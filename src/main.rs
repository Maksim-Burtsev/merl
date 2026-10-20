mod app;
mod buffer;
mod clock;
mod for_agents;
mod git;
mod intraline;
mod key;
mod line_edit;
mod live;
mod markdown;
mod mermaid;
mod picker;
mod picture;
mod real;
mod reviews;
mod search;
mod stats;
mod theme;
mod tree;
mod treesitter;
mod tutor;
mod ui;
#[cfg(target_family = "wasm")]
mod web;
mod wrap;

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::io::{Write, stdout};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
#[cfg(not(target_family = "wasm"))]
use ratatui::crossterm::cursor::{MoveTo, Show};
#[cfg(not(target_family = "wasm"))]
use ratatui::crossterm::event::{
    DisableBracketedPaste, EnableBracketedPaste, Event, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
#[cfg(not(target_family = "wasm"))]
use ratatui::crossterm::terminal::{Clear, ClearType};
#[cfg(not(target_family = "wasm"))]
use ratatui::crossterm::{execute, terminal};

use crate::app::{App, Focus};
#[cfg(not(target_family = "wasm"))]
use crate::buffer::Buffer;
use crate::picker::PickItem;
use crate::tutor::{Drill, Tutor};

enum Msg {
    Key(key::KeyEvent),
    Paste(String),
    Resize,
    Fs(notify::Event),
    ProjectWalked(tree::Tree, Vec<PathBuf>),
    ReviewListed(Result<git::Review, String>),
    Marks(HashMap<PathBuf, char>),
    DiffDone(PathBuf, git::Diff),
    SearchDone { seq: u64, rows: Vec<PickItem> },
    Diagram(mermaid::Done),
    Quit,
}

#[derive(Parser)]
#[command(name = "merl", version, about)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is a command-line switch of its own"
)]
struct Cli {
    #[arg(help = "Directory, file, or FILE:LINE[:COL] to open (default: the current directory)")]
    target: Option<String>,
    #[arg(short, long, value_name = "NAME", help = "Colour theme")]
    theme: Option<String>,
    #[arg(
        long,
        help = "Walk through every key on a bundled sample project (ignores the target)"
    )]
    tutor: bool,
    #[arg(
        help = "Train the keys you do not press: N tasks (default 20) on the sample project, no \
                key named",
        short,
        long,
        value_name = "N",
        num_args = 0..=1,
        default_missing_value = "20",
        value_parser = clap::value_parser!(u16).range(1..)
    )]
    drill: Option<u16>,
    #[arg(
        short,
        long,
        help = "Print how often each key has been pressed and missed, the unused last, and exit"
    )]
    keys: bool,
    #[arg(
        long,
        help = "Print your review sessions of the last 30 days, the time each took and its \
                excursions, and exit"
    )]
    reviews: bool,
    #[arg(
        help = "Review the checked-out branch (or `--review BRANCH` to fetch it and check out \
                what was pushed): its files in the panel, its diff over the code, c / C between \
                hunks",
        short,
        long,
        value_name = "BRANCH",
        num_args = 0..=1,
        default_missing_value = ""
    )]
    review: Option<String>,
    #[arg(
        short,
        long,
        value_name = "REF",
        requires = "review",
        help = "The branch the review is against (default: origin/HEAD, then the remote's \
                master, main or develop, then the local master or main)"
    )]
    base: Option<String>,
    #[arg(long, hide = true)]
    for_agents: bool,
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    if let Err(e) = run() {
        eprintln!("merl: {e:#}");
        std::process::exit(1);
    }
}

fn report(cli: &Cli) -> Result<Option<String>> {
    if cli.keys {
        let path = stats::path().context("no home directory")?;
        return Ok(Some(stats::report(&path, stats::utc_today())?));
    }
    if cli.for_agents {
        return Ok(Some(for_agents::guide()));
    }
    if cli.reviews {
        let path = reviews::path().context("no home directory")?;
        return Ok(Some(reviews::report(&path, stats::utc_today())?));
    }
    Ok(None)
}

fn open_review(
    cli: &Cli,
    root: &mut PathBuf,
    file: &mut Option<PathBuf>,
) -> Result<(Option<git::Review>, Option<git::Opened>)> {
    let Some(branch) = &cli.review else {
        return Ok((None, None));
    };
    *root = git_toplevel(root).context("--review needs a git repository")?;
    let branch = Some(branch.as_str()).filter(|b| !b.is_empty());
    let base = cli.base.as_deref();
    let (at, r) = match branch {
        Some(b) => match git::worktree_of(root, b) {
            Some(wt) => (wt.clone(), git::Review::open(&wt, None, base)?),
            None => git::Review::open_branch(root, b, base)?,
        },
        None => (root.clone(), git::Review::open(root, None, base)?),
    };
    let opened = git::Opened::mark(&at);
    let top = root.clone();
    std::thread::spawn(move || git::sweep_folders(&top, std::time::SystemTime::now()));
    if at != *root {
        let rel = |f: PathBuf| Some(at.join(f.strip_prefix(&*root).ok()?));
        *file = file.take().and_then(rel).filter(|f| f.is_file());
        *root = at;
    }
    Ok((Some(r), opened))
}

fn configure(app: &mut App, config: &theme::Config, name: String) {
    app.autosave = Duration::from_millis(config.autosave_delay_ms);
    app.review_panel_colours = config.review_panel_colours;
    app.review_list_marks = config.review_list_marks;
    app.review_open_files_first = config.review_open_files_first;
    app.clock_format = config.clock.clone();
    app.notes.on = config.review_notes;
    app.theme = name;
    app.config = theme::config_path();
}

fn start_tutor(app: &mut App, drill: Option<u16>, dir: PathBuf) -> Result<()> {
    app.show_tree = false;
    app.focus = Focus::Code;
    let drill = match drill {
        Some(n) => {
            let keys = stats::path();
            let log = keys.as_ref().map(|k| k.with_file_name("drill.tsv"));
            Some(Drill::new(
                n.into(),
                keys.as_deref(),
                log,
                stats::utc_today(),
            )?)
        }
        None => None,
    };
    app.tutor = Some(Tutor {
        step: 0,
        dir,
        drill,
    });
    tutor::begin(app)
}

#[cfg(not(target_family = "wasm"))]
fn run() -> Result<()> {
    let cli = Cli::parse();
    if let Some(text) = report(&cli)? {
        print_even_to_a_closed_pipe(&text);
        return Ok(());
    }
    let config = theme::config()?;
    let (theme, name) = match cli.theme.clone() {
        Some(name) => (theme::load(&name)?, name),
        None => (theme::load_configured(&config.theme)?, config.theme.clone()),
    };

    let Opened {
        project_root: mut root,
        shallow,
        mut file,
        at: line,
    } = if cli.tutor || cli.drill.is_some() {
        Opened {
            project_root: tutor::unpack_to_temp_dir()?,
            shallow: false,
            file: None,
            at: None,
        }
    } else {
        resolve(cli.target.as_deref())?
    };
    let diagrams = mermaid::Diagrams::detect();
    let (review, pushed_folder) = open_review(&cli, &mut root, &mut file)?;
    let git_dirs = git::dirs(&root);
    let tree_order = (git_toplevel(&root))
        .zip(git_dirs.clone())
        .filter(|_| tree::orders_on() && review.is_none())
        .map(|(top, dirs)| tree::OrderFile {
            file: dirs.common_refs_dir.join("merl").join("tree"),
            under: root
                .strip_prefix(&top)
                .unwrap_or_else(|_| Path::new(""))
                .to_path_buf(),
        });
    let order = tree_order
        .as_ref()
        .map(tree::OrderFile::read)
        .unwrap_or_default();
    let (walked, files) = tree::build_ordered(&root, shallow, &order);
    let project = live::Project::new(&root, shallow, &walked);
    let ignored = walked.ignored_files();
    let tree = match &review {
        Some(r) => tree::from_listing(&r.files.iter().map(|f| f.path.clone()).collect::<Vec<_>>()),
        None => walked,
    };
    let buf = match &file {
        Some(p) => Buffer::load(p)?,
        None => Buffer::empty(),
    };
    let dir = root.clone();
    let mut app = App::new(root, tree, files, buf, line);
    app.diagrams = diagrams;
    app.shallow = shallow;
    app.tree_order = tree_order;
    app.ignored = ignored;
    app.pushed_folder = pushed_folder;
    if !(cli.tutor || cli.drill.is_some()) {
        app.recent_file = git_dirs.map(|d| d.worktree_head_dir.join("merl/recent"));
    }
    app.note_recent();
    if let Some(r) = review {
        app.start_review(r);
    }
    configure(&mut app, &config, name);
    if cli.tutor || cli.drill.is_some() {
        start_tutor(&mut app, cli.drill, dir)?;
    }

    let (mut terminal, keyboard) = enter_terminal()?;
    mermaid::keep_panics_inside();
    let (tx, rx) = mpsc::channel();
    spawn_key_reader(&keyboard, tx.clone());
    quit_on_signals(tx.clone());
    let result = event_loop(&mut terminal, &mut app, theme, &rx, tx, project);
    drop(app.diagrams.clear(&mut stdout()));
    leave_terminal(&keyboard);
    if let Some(t) = &app.tutor {
        remove_sample_project_and_print_drill(t);
    }
    if let Some(line) = app.notes_farewell() {
        println!("{line}");
    }
    record_presses_and_reviews(&mut app);
    result
}

fn print_even_to_a_closed_pipe(text: &str) {
    drop(stdout().write_all(text.as_bytes()));
}

struct KeyboardQueried {
    enhanced: bool,
}

#[cfg(not(target_family = "wasm"))]
fn enter_terminal() -> Result<(ratatui::DefaultTerminal, KeyboardQueried)> {
    let terminal = ratatui::try_init()?;
    drop(execute!(stdout(), EnableBracketedPaste));
    let enhanced = terminal::supports_keyboard_enhancement().unwrap_or(false);
    if enhanced {
        execute!(
            stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
            )
        )?;
    }
    restore_on_panic(enhanced);
    Ok((terminal, KeyboardQueried { enhanced }))
}

#[cfg(not(target_family = "wasm"))]
fn restore_on_panic(enhanced: bool) {
    let ratatui_restore = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if enhanced {
            drop(execute!(stdout(), PopKeyboardEnhancementFlags));
        }
        ratatui_restore(info);
        drop(execute!(stdout(), Show));
    }));
}

#[cfg(not(target_family = "wasm"))]
fn spawn_key_reader(_after: &KeyboardQueried, keys: Sender<Msg>) {
    std::thread::spawn(move || {
        while let Ok(ev) = ratatui::crossterm::event::read() {
            let msg = match ev {
                Event::Key(k) => Msg::Key(k),
                Event::Paste(text) => Msg::Paste(text),
                Event::Resize(..) => Msg::Resize,
                _ => continue,
            };
            if keys.send(msg).is_err() {
                break;
            }
        }
    });
}

#[cfg(not(target_family = "wasm"))]
fn quit_on_signals(quit: Sender<Msg>) {
    if let Ok(mut signals) = signal_hook::iterator::Signals::new([
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGHUP,
        signal_hook::consts::SIGINT,
    ]) {
        std::thread::spawn(move || {
            if signals.forever().next().is_some() {
                drop(quit.send(Msg::Quit));
            }
        });
    }
}

#[cfg(not(target_family = "wasm"))]
fn leave_terminal(keyboard: &KeyboardQueried) {
    undo_what_merl_set_while_still_on_the_alternate_screen(&mut stdout(), keyboard);
    ratatui::restore();
    let show_cursor_the_last_frame_hid = Show;
    drop(execute!(stdout(), show_cursor_the_last_frame_hid));
}

#[cfg(not(target_family = "wasm"))]
fn undo_what_merl_set_while_still_on_the_alternate_screen(
    out: &mut impl Write,
    keyboard: &KeyboardQueried,
) {
    if keyboard.enhanced {
        drop(execute!(out, PopKeyboardEnhancementFlags));
    }
    drop(execute!(
        out,
        DisableBracketedPaste,
        Clear(ClearType::All),
        MoveTo(0, 0)
    ));
}

fn remove_sample_project_and_print_drill(tutor: &Tutor) {
    drop(std::fs::remove_dir_all(&tutor.dir));
    if let Some(d) = &tutor.drill {
        print!("{}", d.summary());
    }
}

fn record_presses_and_reviews(app: &mut App) {
    if let Some(path) = stats::path()
        && let Err(e) = stats::add(&path, stats::utc_today(), &app.pressed, &app.missed)
    {
        eprintln!("merl: {e:#}");
    }
    if let Some(path) = reviews::path() {
        for (repo, branch, columns) in app.review_rows() {
            if let Err(e) = reviews::add(&path, stats::utc_today(), &repo, &branch, &columns) {
                eprintln!("merl: {e:#}");
            }
        }
    }
}

struct Watching {
    project: live::Project,
    project_watcher: Option<RecommendedWatcher>,
    project_watched: HashSet<PathBuf>,
    repo: Option<live::Repo>,
    tmux: Option<Sender<String>>,
}

impl Watching {
    fn start(app: &App, project: live::Project, fs: &Sender<Msg>) -> Self {
        let mut project_watcher = watcher(fs.clone());
        let mut project_watched = HashSet::new();
        if let Some(w) = &mut project_watcher {
            project.watch(w, &mut project_watched);
        }
        let mut repo = git::dirs(&app.root).map(|dirs| live::Repo::new(&dirs));
        if let (Some(r), Some(w)) = (&repo, &mut project_watcher) {
            r.watch(w);
        }
        if let Some(r) = repo.as_mut().filter(|_| app.review.is_none()) {
            r.listing();
            list_marks(fs.clone(), app.root.clone());
        }
        Self {
            project,
            project_watcher,
            project_watched,
            repo,
            tmux: spawn_tmux_copier(),
        }
    }

    fn spawn_jobs(&mut self, app: &mut App, diff_tx: &Sender<Msg>) {
        if std::mem::take(&mut app.want_diff)
            && let Some(path) = app.buf.path.clone()
        {
            let (tx, root) = (diff_tx.clone(), app.root.clone());
            std::thread::spawn(move || {
                let diff = git::diff(&root, &path, None, None);
                drop(tx.send(Msg::DiffDone(path, diff)));
            });
        }
        if self.project.walk_due(Instant::now()) {
            let (tx, root, shallow) = (diff_tx.clone(), app.root.clone(), app.shallow);
            let order = app.tree_order.clone();
            std::thread::spawn(move || {
                let order = order
                    .as_ref()
                    .map(tree::OrderFile::read)
                    .unwrap_or_default();
                let (tree, files) = tree::build_ordered(&root, shallow, &order);
                drop(tx.send(Msg::ProjectWalked(tree, files)));
            });
        }
        if let Some(live) = &mut self.repo
            && live.list_due(Instant::now())
        {
            let (tx, root) = (diff_tx.clone(), app.root.clone());
            match app.review.clone() {
                Some(r) => {
                    std::thread::spawn(move || {
                        let fresh = r.refresh(&root).map_err(|e| app::error_text(&e));
                        drop(tx.send(Msg::ReviewListed(fresh)));
                    });
                }
                None => list_marks(tx, root),
            }
        }
        if let Some(job) = app.search_tick() {
            let tx = diff_tx.clone();
            std::thread::spawn(move || {
                drop(tx.send(Msg::SearchDone {
                    seq: job.seq,
                    rows: job.items(),
                }));
            });
        }
    }

    fn key(&mut self, app: &mut App, k: key::KeyEvent) -> ControlFlow<(), bool> {
        if app.key(k) {
            return ControlFlow::Break(());
        }
        let open_ignored_dirs_moved = self.project.shown(&app.tree);
        if open_ignored_dirs_moved && let Some(w) = &mut self.project_watcher {
            self.project.watch(w, &mut self.project_watched);
        }
        if let Some(text) = app.clipboard.take() {
            osc52_copy(&text);
            if let Some(tmux) = &self.tmux {
                drop(tmux.send(text));
            }
        }
        ControlFlow::Continue(true)
    }

    fn on_msg(
        &mut self,
        app: &mut App,
        msg: Result<Msg, mpsc::RecvTimeoutError>,
    ) -> ControlFlow<(), bool> {
        let dirty = match msg {
            Ok(Msg::Key(k)) => return self.key(app, k),
            Ok(Msg::Paste(text)) => {
                app.paste(&text);
                true
            }
            Ok(Msg::DiffDone(path, diff)) => {
                let shown = app.buf.path == Some(path) && app.review.is_none();
                if shown {
                    app.diff = diff;
                }
                shown
            }
            Ok(Msg::Quit) => {
                app.flush();
                return ControlFlow::Break(());
            }
            Ok(Msg::SearchDone { seq, rows }) => app.search_done(seq, rows),
            Ok(Msg::Resize) => {
                app.diagrams.resized();
                true
            }
            Ok(Msg::Diagram(done)) => {
                app.diagrams.done(done);
                true
            }
            Ok(Msg::Fs(ev)) => {
                let dirty = concerns_open_file(app, &ev) && app.reload(false);
                self.project.event(&ev, Instant::now());
                if let Some(r) = &mut self.repo {
                    r.event(&ev, self.project.touched(&ev), Instant::now());
                }
                dirty
            }
            Ok(Msg::ProjectWalked(tree, files)) => {
                self.project.walked(&tree, Instant::now());
                app.project_walked(tree, files);
                self.project.shown(&app.tree);
                if let Some(w) = &mut self.project_watcher {
                    self.project.watch(w, &mut self.project_watched);
                }
                if let Some(r) = &mut self.repo {
                    r.touch(Instant::now());
                }
                true
            }
            Ok(Msg::Marks(marks)) => {
                if let Some(r) = &mut self.repo {
                    r.listed();
                }
                let changed = app.marks != marks;
                app.marks = marks;
                changed
            }
            Ok(Msg::ReviewListed(fresh)) => {
                if let Some(r) = &mut self.repo {
                    r.listed();
                }
                match fresh {
                    Ok(fresh) => app.review_refreshed(fresh),
                    Err(e) => {
                        app.message = format!("review not refreshed: {e}");
                        true
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => app.diagrams.wake.is_some() | app.parse_more(),
            Err(mpsc::RecvTimeoutError::Disconnected) => return ControlFlow::Break(()),
        };
        ControlFlow::Continue(dirty)
    }
}

fn restyle(app: &mut App, theme: &mut theme::Theme, drawn_theme: &mut String) -> bool {
    if app.shown_theme() == drawn_theme.as_str() {
        return false;
    }
    app.shown_theme().clone_into(drawn_theme);
    match theme::load(drawn_theme) {
        Ok(t) => {
            *theme = t;
            app.buf.clear_hl();
            if let Some((_, base)) = &mut app.base {
                base.clear_hl();
            }
        }
        Err(e) => app.message = app::error_text(&e),
    }
    true
}

#[cfg(not(target_family = "wasm"))]
fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    mut theme: theme::Theme,
    rx: &mpsc::Receiver<Msg>,
    fs: Sender<Msg>,
    project: live::Project,
) -> Result<()> {
    let diff_tx = fs.clone();
    let mut drawn_theme = app.theme.clone();
    let mut open_file_dir_watcher = watcher(fs);
    app.no_watch = open_file_dir_watcher.is_none();
    let mut watched: Option<(PathBuf, PathBuf)> = None;
    let mut live = Watching::start(app, project, &diff_tx);

    let mut dirty = true;
    loop {
        if drill_answered(app) {
            return Ok(());
        }
        if let Some(p) = &mut app.picker {
            dirty |= p.tick();
        }
        dirty |= app.tick();
        app.warm_up();
        rewatch(open_file_dir_watcher.as_mut(), &mut watched, app);
        live.spawn_jobs(app, &diff_tx);
        dirty |= restyle(app, &mut theme, &mut drawn_theme);
        dirty |= app.tick_clock();
        if dirty {
            terminal.draw(|f| ui::draw(f, app, &theme))?;
            app.diagrams.flush(&mut stdout())?;
            if let Some(at) = app.cursor_cell {
                terminal.set_cursor_position(at)?;
            }
            dirty = false;
        }
        for job in app.diagrams.jobs() {
            let tx = diff_tx.clone();
            drop(
                std::thread::Builder::new()
                    .name(mermaid::THREAD.into())
                    .spawn(move || {
                        drop(tx.send(Msg::Diagram(mermaid::render(job))));
                    }),
            );
        }
        let idle = Duration::from_millis(if app.picker.is_some() { 10 } else { 100 });
        let idle = app.diagrams.wake.map_or(idle, |w| w.min(idle));
        let idle = if app.parsing() { Duration::ZERO } else { idle };
        match live.on_msg(app, rx.recv_timeout(idle)) {
            ControlFlow::Continue(changed) => dirty |= changed,
            ControlFlow::Break(()) => return Ok(()),
        }
    }
}

fn watcher(tx: Sender<Msg>) -> Option<RecommendedWatcher> {
    notify::recommended_watcher(move |ev: notify::Result<notify::Event>| {
        if let Ok(ev) = ev {
            drop(tx.send(Msg::Fs(ev)));
        }
    })
    .ok()
}

fn spawn_tmux_copier() -> Option<Sender<String>> {
    let (mut taken, mut calls) = tmux_copy(std::env::var_os("TMUX").as_deref())?;
    let (tx, texts) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        for text in texts {
            hand_to_tmux(&mut taken, &mut calls, &text);
        }
    });
    Some(tx)
}

fn drill_answered(app: &App) -> bool {
    (app.tutor.as_ref()).is_some_and(|t| t.drill.as_ref().is_some_and(Drill::over))
}

fn osc52_copy(text: &str) {
    let mut out = stdout();
    drop(write!(out, "\x1b]52;c;{}\x07", base64(text.as_bytes())));
    drop(out.flush());
}

fn list_marks(tx: Sender<Msg>, root: PathBuf) {
    std::thread::spawn(move || {
        drop(tx.send(Msg::Marks(git::changed_files(&root))));
    });
}

fn rewatch(
    watcher: Option<&mut RecommendedWatcher>,
    watched: &mut Option<(PathBuf, PathBuf)>,
    app: &App,
) {
    let open = app.buf.path.as_deref();
    if open == watched.as_ref().map(|(p, _)| p.as_path()) {
        return;
    }
    let Some(watcher) = watcher else { return };
    if let Some((_, old)) = watched.take() {
        drop(watcher.unwatch(&old));
    }
    let Some(open) = open else { return };
    let real = open.canonicalize().unwrap_or_else(|_| open.to_path_buf());
    if let Some(dir) = real.parent() {
        drop(watcher.watch(dir, RecursiveMode::NonRecursive));
        *watched = Some((open.to_path_buf(), dir.to_path_buf()));
    }
}

fn concerns_open_file(app: &App, ev: &notify::Event) -> bool {
    if matches!(ev.kind, EventKind::Access(_)) {
        return false;
    }
    let Some(open) = app.buf.path.as_deref() else {
        return false;
    };
    let (name, dir) = (open.file_name(), open.parent());
    let (mut real, mut target) = (None, None);
    ev.paths.iter().any(|p| {
        p.file_name() == name
            && (p.parent() == dir
                || p.parent()
                    == real
                        .get_or_insert_with(|| dir?.canonicalize().ok())
                        .as_deref())
            || Some(p.as_path())
                == target
                    .get_or_insert_with(|| open.canonicalize().ok())
                    .as_deref()
    })
}

type LineCol1 = (usize, usize);

#[derive(Debug)]
struct Opened {
    project_root: PathBuf,
    shallow: bool,
    file: Option<PathBuf>,
    at: Option<LineCol1>,
}

fn resolve(target: Option<&str>) -> Result<Opened> {
    let dir = |project_root| Opened {
        project_root,
        shallow: false,
        file: None,
        at: None,
    };
    let Some(target) = target else {
        return Ok(dir(std::env::current_dir()?));
    };
    let (path, line) = split_line(target);
    let path = PathBuf::from(path);
    if path.is_dir() {
        return Ok(dir(path.canonicalize()?));
    }
    if !path.exists() {
        anyhow::bail!("{}: no such file or directory", path.display());
    }
    if !path.is_file() {
        anyhow::bail!("{}: not a regular file", path.display());
    }
    let path = path
        .canonicalize()
        .with_context(|| format!("{}", path.display()))?;
    let dir = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let (project_root, shallow) = git_toplevel(&dir).map_or((dir, true), |top| (top, false));
    Ok(Opened {
        project_root,
        shallow,
        file: Some(path),
        at: line,
    })
}

fn split_line(target: &str) -> (&str, Option<LineCol1>) {
    if Path::new(target).exists() {
        return (target, None);
    }
    let found = target
        .rmatch_indices(':')
        .filter_map(|(i, _)| Some((target.get(..i)?, target.get(i + 1..)?)))
        .find(|(path, _)| Path::new(path).exists());
    let Some((path, rest)) = found else {
        return (target, None);
    };
    let mut nums = rest.split(':').map(|n| n.parse().ok());
    let line = nums.next().flatten();
    let col = nums.next().flatten().unwrap_or(1);
    (path, line.map(|n| (n, col)))
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[(n >> (18 - 6 * i)) as usize & 63] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn tmux_copy(tmux: Option<&OsStr>) -> Option<(Command, [Command; 2])> {
    tmux.filter(|t| !t.is_empty())?;
    let tmux = |args: &[&str], stdin: Stdio, stdout: Stdio| {
        let mut cmd = Command::new("tmux");
        cmd.args(args)
            .stdin(stdin)
            .stdout(stdout)
            .stderr(Stdio::null());
        cmd
    };
    let taken = tmux(
        &["show-options", "-sv", "set-clipboard"],
        Stdio::null(),
        Stdio::piped(),
    );
    let calls = [&["load-buffer", "-w", "-"][..], &["load-buffer", "-"]]
        .map(|args| tmux(args, Stdio::piped(), Stdio::null()));
    Some((taken, calls))
}

fn hand_to_tmux(taken: &mut Command, calls: &mut [Command], text: &str) {
    if taken.output().is_ok_and(|o| o.stdout.trim_ascii() == b"on") {
        return;
    }
    for call in calls {
        let Ok(mut child) = call.spawn() else { break };
        if let Some(mut stdin) = child.stdin.take() {
            drop(stdin.write_all(text.as_bytes()));
        }
        if child.wait().is_ok_and(|s| s.success()) {
            break;
        }
    }
}

fn git_toplevel(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|d| d.join(".git").exists())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn base64_matches_the_standard_alphabet_and_padding() {
        assert_eq!(super::base64(b""), "");
        assert_eq!(super::base64(b"f"), "Zg==");
        assert_eq!(super::base64(b"fo"), "Zm8=");
        assert_eq!(super::base64(b"foo"), "Zm9v");
        assert_eq!(super::base64("hi\nтам".as_bytes()), "aGkK0YLQsNC8");
    }

    #[test]
    fn leaving_clears_the_screen_and_homes_the_cursor_for_a_terminal_without_alternate_screen() {
        for enhanced in [false, true] {
            let mut out = Vec::new();
            super::undo_what_merl_set_while_still_on_the_alternate_screen(
                &mut out,
                &super::KeyboardQueried { enhanced },
            );
            let out = String::from_utf8(out).unwrap();
            assert!(out.ends_with("\x1b[2J\x1b[1;1H"), "{out:?}");
            assert_eq!(out.starts_with("\x1b[<1u"), enhanced, "{out:?}");
        }
    }

    #[test]
    fn inside_tmux_a_copy_goes_to_load_buffer_on_stdin() {
        use std::ffi::OsStr;
        assert!(super::tmux_copy(None).is_none());
        assert!(super::tmux_copy(Some(OsStr::new(""))).is_none());
        let (taken, calls) =
            super::tmux_copy(Some(OsStr::new("/tmp/tmux-501/default,4242,0"))).unwrap();
        let line = |c: &std::process::Command| {
            let args: Vec<_> = c.get_args().map(|a| a.to_str().unwrap()).collect();
            format!("{} {}", c.get_program().display(), args.join(" "))
        };
        assert_eq!(line(&taken), "tmux show-options -sv set-clipboard");
        assert_eq!(
            calls.each_ref().map(line),
            ["tmux load-buffer -w -", "tmux load-buffer -"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_copy_goes_to_the_first_call_that_works_unless_tmux_took_the_osc_52() {
        use std::process::{Command, Stdio};
        let got = std::env::temp_dir().join(format!("merl-tmux-copy-{}", std::process::id()));
        drop(std::fs::remove_file(&got));
        let sh = |script: &str| {
            let mut cmd = Command::new("sh");
            cmd.args(["-c", script]).stdin(Stdio::piped());
            cmd
        };
        let mut calls = [sh("exit 1"), sh(&format!("cat > '{}'", got.display()))];
        super::hand_to_tmux(&mut sh("echo external"), &mut calls, "one");
        assert_eq!(std::fs::read_to_string(&got).unwrap(), "one");
        std::fs::remove_file(&got).unwrap();
        super::hand_to_tmux(&mut sh("echo on"), &mut calls, "two");
        assert!(!got.exists());
    }

    #[test]
    fn the_readme_names_the_rust_version_cargo_toml_sets() {
        let wanted = format!(
            "Or build it (Rust {} or newer):",
            env!("CARGO_PKG_RUST_VERSION")
        );
        let readme = include_str!("../README.md");
        assert!(readme.contains(&wanted), "README.md should say {wanted:?}");
    }

    #[test]
    fn the_help_of_base_names_the_bases_merl_tries() {
        use clap::CommandFactory;
        assert_eq!(
            crate::git::BASES,
            [
                "origin/master",
                "origin/main",
                "origin/develop",
                "master",
                "main"
            ]
        );
        let cli = super::Cli::command();
        let base = cli.get_arguments().find(|a| a.get_id() == "base").unwrap();
        assert_eq!(
            base.get_help().unwrap().to_string(),
            "The branch the review is against (default: origin/HEAD, then the remote's master, \
             main or develop, then the local master or main)"
        );
    }

    #[test]
    fn review_takes_the_word_after_it_as_its_branch() {
        use clap::Parser;
        let review = |args: &[&str]| {
            let cli = super::Cli::parse_from(args);
            (cli.review, cli.base, cli.target)
        };
        let s = |s: &str| Some(s.to_owned());
        assert_eq!(review(&["merl", "--review"]), (s(""), None, None));
        assert_eq!(
            review(&["merl", "--review", "feature"]),
            (s("feature"), None, None)
        );
        assert_eq!(
            review(&["merl", "--review", "feature", "src/main.rs"]),
            (s("feature"), None, s("src/main.rs"))
        );
        assert_eq!(
            review(&["merl", "--review", "--base", "dev"]),
            (s(""), s("dev"), None)
        );
        assert_eq!(
            review(&["merl", "--review=feature", "--base=dev"]),
            (s("feature"), s("dev"), None)
        );
        assert!(super::Cli::try_parse_from(["merl", "--base", "x"]).is_err());
        let cli = super::Cli::parse_from(["merl", "--reviews"]);
        assert_eq!((cli.reviews, cli.review), (true, None));
    }

    #[test]
    fn short_flags_read_as_the_long_ones() {
        use clap::Parser;
        let cli =
            super::Cli::parse_from(["merl", "-r", "feature", "-b", "origin/dev", "-t", "nord"]);
        assert_eq!(
            (
                cli.review.as_deref(),
                cli.base.as_deref(),
                cli.theme.as_deref()
            ),
            (Some("feature"), Some("origin/dev"), Some("nord"))
        );
        let cli = super::Cli::parse_from(["merl", "-r", "-b", "dev"]);
        assert_eq!(
            (cli.review.as_deref(), cli.base.as_deref()),
            (Some(""), Some("dev"))
        );
        assert_eq!(super::Cli::parse_from(["merl", "-d"]).drill, Some(20));
        assert_eq!(super::Cli::parse_from(["merl", "-d", "5"]).drill, Some(5));
        assert!(super::Cli::parse_from(["merl", "-k"]).keys);
        assert!(super::Cli::try_parse_from(["merl", "-b", "x"]).is_err());
    }

    #[test]
    fn a_directory_that_cannot_be_watched_is_not_tried_again() {
        use notify::{RecursiveMode, Watcher};
        let dir = std::env::temp_dir().join(format!("merl-rewatch-{}", std::process::id()));
        let gone = dir.join("gone");
        let buf = crate::buffer::Buffer::from_bytes(gone.join("x.rs"), b"x\n");
        let app = crate::app::App::new(dir, crate::tree::Tree::default(), Vec::new(), buf, None);
        let mut watcher =
            notify::recommended_watcher(|_: notify::Result<notify::Event>| {}).unwrap();
        assert!(watcher.watch(&gone, RecursiveMode::NonRecursive).is_err());
        let mut watched = None;
        super::rewatch(Some(&mut watcher), &mut watched, &app);
        assert_eq!(
            watched.as_ref().map(|(_, dir)| dir.as_path()),
            Some(gone.as_path())
        );
    }

    #[test]
    fn a_namesake_elsewhere_in_the_project_is_not_the_open_file() {
        use notify::{Event, EventKind, event::CreateKind};
        let dir = std::env::temp_dir().join(format!("merl-namesake-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("node_modules/pkg")).unwrap();
        std::fs::write(dir.join("index.js"), "x\n").unwrap();
        let buf = crate::buffer::Buffer::load(&dir.join("index.js")).unwrap();
        let app = crate::app::App::new(
            dir.clone(),
            crate::tree::Tree::default(),
            Vec::new(),
            buf,
            None,
        );
        let real = dir.canonicalize().unwrap();
        for (path, want) in [
            (dir.join("index.js"), true),
            (real.join("index.js"), true),
            (real.join("node_modules/pkg/index.js"), false),
            (real.join("main.js"), false),
        ] {
            let ev = Event::new(EventKind::Create(CreateKind::File)).add_path(path.clone());
            assert_eq!(super::concerns_open_file(&app, &ev), want, "{path:?}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_file_link_reloads_when_the_file_it_leads_to_changes() {
        use notify::{Event, EventKind, event::ModifyKind};
        let dir = std::env::temp_dir().join(format!("merl-link-watch-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(dir.join("proj")).unwrap();
        std::fs::create_dir_all(dir.join("team-rules")).unwrap();
        std::fs::write(dir.join("team-rules/AGENTS.md"), "x\n").unwrap();
        std::os::unix::fs::symlink("../team-rules/AGENTS.md", dir.join("proj/team.md")).unwrap();
        let buf = crate::buffer::Buffer::load(&dir.join("proj/team.md")).unwrap();
        let app = crate::app::App::new(
            dir.join("proj"),
            crate::tree::Tree::default(),
            Vec::new(),
            buf,
            None,
        );
        let mut watcher =
            notify::recommended_watcher(|_: notify::Result<notify::Event>| {}).unwrap();
        let mut watched = None;
        super::rewatch(Some(&mut watcher), &mut watched, &app);
        let real = dir.canonicalize().unwrap();
        assert_eq!(
            watched.map(|(_, d)| d),
            Some(real.join("team-rules")),
            "the folder of the file the link leads to is watched (#500)"
        );
        for (path, want) in [
            (real.join("team-rules/AGENTS.md"), true),
            (real.join("team-rules/README.md"), false),
        ] {
            let ev = Event::new(EventKind::Modify(ModifyKind::Any)).add_path(path.clone());
            assert_eq!(super::concerns_open_file(&app, &ev), want, "{path:?}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_file_outside_a_repository_lists_its_neighbours_not_all_below_them() {
        let dir = std::env::temp_dir().join(format!("merl-no-repo-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(dir.join("deep/er")).unwrap();
        for f in ["a.txt", "b.txt", "deep/c.txt", "deep/er/d.txt"] {
            std::fs::write(dir.join(f), "x\n").unwrap();
        }
        let real = dir.canonicalize().unwrap();
        let walk = |target: &Path| {
            let super::Opened {
                project_root: root,
                shallow,
                file,
                ..
            } = super::resolve(target.to_str()).unwrap();
            assert_eq!(root, real);
            (file, crate::tree::build(&root, shallow).1)
        };
        let (file, files) = walk(&dir.join("a.txt"));
        assert_eq!(file, Some(real.join("a.txt")));
        assert_eq!(files, [Path::new("a.txt"), Path::new("b.txt")]);
        assert_eq!(
            walk(&dir).1.len(),
            4,
            "a named directory is walked all the way down"
        );
        std::fs::create_dir(dir.join(".git")).unwrap();
        assert_eq!(
            walk(&dir.join("deep/c.txt")).1.len(),
            4,
            "a file in a repository lists the whole repository"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn a_fifo_is_refused_before_it_is_read() {
        let dir = std::env::temp_dir().join(format!("merl-fifo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pipe = dir.join("pipe");
        let mkfifo = std::process::Command::new("mkfifo").arg(&pipe).status();
        assert!(mkfifo.unwrap().success());
        let err = super::resolve(pipe.to_str()).unwrap_err().to_string();
        assert_eq!(err, format!("{}: not a regular file", pipe.display()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn splits_what_compilers_and_grep_print() {
        use super::split_line;
        let main = "src/main.rs";
        assert_eq!(split_line("src/main.rs:20"), (main, Some((20, 1))));
        assert_eq!(split_line("src/main.rs:12:5"), (main, Some((12, 5))));
        assert_eq!(split_line("src/main.rs:12:"), (main, Some((12, 1))));
        assert_eq!(split_line("src/main.rs:"), (main, None));
        assert_eq!(split_line("src/main.rs"), (main, None));
        assert_eq!(split_line("src/main.rs:12:mod app;"), (main, Some((12, 1))));
        assert_eq!(
            split_line("src/main.rs:16:5:use std::collections::HashSet;"),
            (main, Some((16, 5)))
        );
        assert_eq!(split_line("nope:12"), ("nope:12", None));
        assert_eq!(split_line("nope.rs:12:5"), ("nope.rs:12:5", None));
    }

    #[test]
    fn a_colon_in_a_name_that_exists_is_part_of_the_name() {
        use super::split_line;
        let dir = std::env::temp_dir().join(format!("merl-colon-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a:12").display().to_string();
        std::fs::write(&file, "x\n").unwrap();
        assert_eq!(split_line(&file), (file.as_str(), None));
        assert_eq!(
            split_line(&format!("{file}:3:4")),
            (file.as_str(), Some((3, 4)))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    const LONG_FILES: &[(&str, usize)] = &[];
    const MAX_LINES: usize = 1500;

    fn rust_files(dir: &Path, with_tests: bool, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if with_tests || !path.ends_with("tests") {
                    rust_files(&path, with_tests, out);
                }
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    #[test]
    fn no_source_file_grows_past_its_size() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        rust_files(&root.join("src"), false, &mut files);
        let mut wrong = Vec::new();
        for path in files {
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\\', "/");
            let text = std::fs::read_to_string(&path).unwrap();
            let lines: Vec<&str> = text.lines().collect();
            let len = lines
                .windows(2)
                .position(|w| {
                    w[0] == "#[cfg(test)]" && w[1].starts_with("mod ") && w[1].ends_with('{')
                })
                .unwrap_or(lines.len());
            let allowed = LONG_FILES.iter().find(|(f, _)| *f == name).map(|&(_, n)| n);
            match allowed {
                None if len > MAX_LINES => wrong.push(format!(
                    "{name} has {len} lines, over {MAX_LINES}: split it into modules"
                )),
                Some(n) if len > n => wrong.push(format!(
                    "{name} grew from {n} to {len} lines: split it, do not raise its number in LONG_FILES"
                )),
                Some(_) if len <= MAX_LINES => wrong.push(format!(
                    "{name} is down to {len} lines: take it off LONG_FILES"
                )),
                Some(n) if len < n => wrong.push(format!(
                    "{name} shrank from {n} to {len} lines: lower its number in LONG_FILES to {len}"
                )),
                _ => {}
            }
        }
        for (f, _) in LONG_FILES {
            if !root.join(f).exists() {
                wrong.push(format!("{f} is gone: take it off LONG_FILES"));
            }
        }
        assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    }

    const EXPECTS: usize = 156;

    fn expected_lints(text: &str) -> usize {
        let b = text.as_bytes();
        text.match_indices("expect(")
            .filter(|&(at, m)| {
                at > 0
                    && b"[( \n".contains(&b[at - 1])
                    && b[at + m.len()..]
                        .iter()
                        .find(|c| !c.is_ascii_whitespace())
                        .is_some_and(u8::is_ascii_lowercase)
            })
            .map(|(at, m)| {
                let args = text.get(at + m.len()..).unwrap_or_default();
                let lints = args.split([')', '=']).next().unwrap_or_default();
                lints
                    .split(',')
                    .filter(|l| l.trim() != "reason" && !l.trim().is_empty())
                    .count()
            })
            .sum()
    }

    #[test]
    fn expects_do_not_grow() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        rust_files(&root.join("src"), true, &mut files);
        let count: usize = files
            .iter()
            .map(|p| expected_lints(&std::fs::read_to_string(p).unwrap()))
            .sum();
        assert!(
            count <= EXPECTS,
            "{count} lints silenced under src/, over EXPECTS = {EXPECTS}: fix the code, or give the expect a reason the reviewer will accept"
        );
    }

    #[test]
    fn expected_lints_count_each_lint_not_calls() {
        let text = "#![\x65xpect(clippy::todo)]\n#[cfg_attr(test, \x65xpect(dead_code))]\n\
                    #[\x65xpect(\n    clippy::x,\n    clippy::y,\n    reason = \"r\"\n)]\nlet v = o.\x65xpect(\"why\");\n";
        assert_eq!(expected_lints(text), 4);
    }

    fn skip_block_comment(b: &[u8], i: &mut usize, line: &mut usize, mark: &mut impl FnMut(usize)) {
        let mut depth = 0;
        loop {
            mark(*line);
            if b[*i..].starts_with(b"/*") {
                depth += 1;
                *i += 1;
            } else if b[*i..].starts_with(b"*/") {
                depth -= 1;
                *i += 1;
                if depth == 0 {
                    break;
                }
            } else if b[*i] == b'\n' {
                *line += 1;
            }
            *i += 1;
            if *i >= b.len() {
                break;
            }
        }
    }

    #[expect(
        clippy::naive_bytecount,
        reason = "the bytecount crate would be a dependency for one count of newlines"
    )]
    fn comment_lines(text: &str) -> Vec<usize> {
        let b = text.as_bytes();
        let (mut i, mut line, mut lines) = (0, 0, Vec::new());
        let mut mark = |line: usize| {
            if lines.last() != Some(&(line + 1)) {
                lines.push(line + 1);
            }
        };
        while i < b.len() {
            match b[i] {
                b'\n' => line += 1,
                b'/' if b.get(i + 1) == Some(&b'/') => {
                    mark(line);
                    while b.get(i + 1).is_some_and(|&c| c != b'\n') {
                        i += 1;
                    }
                }
                b'/' if b.get(i + 1) == Some(&b'*') => {
                    skip_block_comment(b, &mut i, &mut line, &mut mark);
                }
                b'"' => {
                    i += 1;
                    while i < b.len() && b[i] != b'"' {
                        if b[i] == b'\\' {
                            i += 1;
                        }
                        line += usize::from(b.get(i) == Some(&b'\n'));
                        i += 1;
                    }
                }
                b'\'' => {
                    let close = if b.get(i + 1) == Some(&b'\\') {
                        b.get(i + 3..)
                            .and_then(|r| r.iter().position(|&c| c == b'\''))
                            .map(|n| i + 3 + n)
                    } else {
                        text.get(i + 1..)
                            .and_then(|r| r.chars().next())
                            .map(|c| i + 1 + c.len_utf8())
                            .filter(|&e| b.get(e) == Some(&b'\''))
                    };
                    if let Some(close) = close {
                        i = close;
                    }
                }
                c if c.is_ascii_alphanumeric() || c == b'_' => {
                    let start = i;
                    while b
                        .get(i + 1)
                        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
                    {
                        i += 1;
                    }
                    let hashes = b[i + 1..].iter().take_while(|&&c| c == b'#').count();
                    if matches!(&b[start..=i], b"r" | b"br" | b"cr")
                        && b.get(i + 1 + hashes) == Some(&b'"')
                    {
                        let end = [&b"\""[..], &b"#".repeat(hashes)].concat();
                        let body = i + 2 + hashes;
                        let close = b[body..]
                            .windows(end.len())
                            .position(|w| w == end)
                            .map_or(b.len(), |n| body + n + end.len() - 1);
                        line += b[i..close].iter().filter(|&&c| c == b'\n').count();
                        i = close;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        lines
    }

    #[test]
    fn comment_lines_count_comments_not_strings() {
        let text = r##"fn f<'a>(s: &'a str) -> char { // one
    /* two
       three /* nested */ */
    let _ = ("// no", r#"// "no" "#, br"/* no", b"\"// no", '"', '\'', '/');
    let _ = "a
// no
b";
    /// four
    'x'
}
"##;
        assert_eq!(comment_lines(text), [1, 2, 3, 8]);
    }

    #[test]
    fn src_holds_no_comment() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        rust_files(&root.join("src"), true, &mut files);
        files.sort();
        let found: Vec<String> = files
            .iter()
            .flat_map(|path| {
                let name = path.strip_prefix(root).unwrap().display().to_string();
                comment_lines(&std::fs::read_to_string(path).unwrap())
                    .into_iter()
                    .map(move |line| format!("{name}:{line}"))
            })
            .collect();
        assert!(
            found.is_empty(),
            "code carries no comments; move the why into the commit message and the what into a \
             name or a test:\n{}",
            found.join("\n")
        );
    }

    #[test]
    fn mutants_skip_the_comment_test_and_only_tests_that_exist() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let flow = std::fs::read_to_string(root.join(".github/workflows/mutants.yml")).unwrap();
        let skips: Vec<&str> = flow
            .split("--skip ")
            .skip(1)
            .filter_map(|s| s.split_whitespace().next())
            .collect();
        assert!(
            skips.contains(&"src_holds_no_comment"),
            "cargo-mutants writes a comment into every mutant, so the comment test catches them all: {skips:?}"
        );
        let mut files = Vec::new();
        rust_files(&root.join("src"), true, &mut files);
        let src: String = files
            .iter()
            .map(|p| std::fs::read_to_string(p).unwrap())
            .collect();
        for name in skips {
            assert!(
                src.contains(&format!("fn {name}(")) || src.contains(&format!("mod {name};")),
                "mutants.yml skips {name}, which names no test"
            );
        }
    }

    #[test]
    fn the_browser_build_has_merl_s_dependencies() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let deps = |file: &str| {
            let text = std::fs::read_to_string(root.join(file)).unwrap();
            let manifest: toml::Table = text.parse().unwrap();
            manifest["dependencies"].as_table().unwrap().clone()
        };
        let (merl, web) = (deps("Cargo.toml"), deps("web/Cargo.toml"));
        let mut wrong: Vec<&String> = (web.keys())
            .filter(|name| !merl.contains_key(*name))
            .collect();
        for (name, spec) in &merl {
            let same = match (name.as_str(), web.get(name)) {
                ("signal-hook", None) => true,
                ("ratatui", Some(w)) => w.get("version") == spec.get("version"),
                (_, w) => w == Some(spec),
            };
            if !same {
                wrong.push(name);
            }
        }
        assert!(
            wrong.is_empty(),
            "web/Cargo.toml builds this code for the browser: give it these dependencies as \
             Cargo.toml has them: {wrong:?}"
        );
    }

    #[test]
    fn mutants_time_the_unmutated_baseline_by_no_fixed_timeout() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let flow = std::fs::read_to_string(root.join(".github/workflows/mutants.yml")).unwrap();
        assert!(
            !flow.contains("--timeout ") && flow.contains("--timeout-multiplier "),
            "cargo-mutants holds the unmutated baseline to a fixed --timeout too, and a suite past it tests no mutant (#868)"
        );
    }

    const USAGE: &str = "the program reads its docstring as `__doc__` and prints it as its usage, \
         so it is behaviour, not a comment; steps 1-6 kept to src/ and never read the tools; without \
         it the tool has no help, which no name or commit message can print";

    const KEPT_COMMENTS: &[(&str, &str, &str)] = &[
        (
            "tools/cast.py",
            r#""""Record merl in a tmux pane as an animated GIF that reads like a screen recording."#,
            USAGE,
        ),
        (
            "tools/nvim2tmtheme.py",
            r#""""Convert a Neovim highlight dump into a TextMate .tmTheme for syntect, and the"#,
            USAGE,
        ),
        (
            "tools/vscode2tmtheme.py",
            r#""""Convert a VS Code JSON colour theme into a TextMate .tmTheme for syntect."#,
            USAGE,
        ),
        (
            "tools/fold-bench/run",
            r#""""The fold bench: `f` pressed on the lines of real projects, against a reference."#,
            USAGE,
        ),
        (
            "tools/fold-bench/record.py",
            r#""""Records the reference of the fold bench: what Neovim folds with nvim-treesitter's `folds.scm`"#,
            USAGE,
        ),
        (
            "tools/theme-bench/record.py",
            r#""""Records the reference of the theme bench: the colours LazyVim paints on the same files, per"#,
            USAGE,
        ),
        (
            "tools/theme-bench/run",
            r#""""The theme bench: merl's colours against LazyVim's, character by character."#,
            USAGE,
        ),
        (
            "tools/d-bench/run",
            r#""""The `d` bench: press `d` on recorded cursors in pinned real projects, score merl's answer"#,
            USAGE,
        ),
        (
            "tools/d-bench/record.py",
            r#""""Recording the `d` bench, once, on a recording machine: never in merl, never in CI."#,
            USAGE,
        ),
        (
            "tests/smoke/run.py",
            r#""""The smoke test: before a release, plays every scenario in tmux on the new build and on the last"#,
            USAGE,
        ),
        (
            "assets/tapes/record.py",
            r#""""Records one of the README's GIFs: a real terminal session, played back at the speed it ran."#,
            USAGE,
        ),
        (
            "tools/shot.py",
            r#""""Renders a `tmux capture-pane -e` dump into a PNG, in the colours of the owner's terminal:"#,
            USAGE,
        ),
    ];

    const SCRIPTS: &[&str] = &[
        "tools",
        "tests/smoke/run.py",
        "tests/smoke/setup.sh",
        "assets/tapes",
        ".claude/hooks",
        ".claude/skills/batch/bin",
        ".claude/skills/groom/bin",
        ".github",
        "Cargo.toml",
        "deny.toml",
        "web/build.sh",
        "web/Cargo.toml",
        "web/review",
    ];
    const SCRIPT_DATA: &[&str] = &[
        "tools/d-bench/answers",
        "tools/fold-bench/folds",
        "tools/themes-src",
        "web/review/main",
        "web/review/branch",
    ];

    fn script_files(root: &Path, name: &str, out: &mut Vec<String>) {
        let path = root.join(name);
        if SCRIPT_DATA.contains(&name) || name.ends_with("__pycache__") {
        } else if path.is_dir() {
            for entry in std::fs::read_dir(&path).unwrap() {
                let file = entry.unwrap().file_name();
                script_files(root, &format!("{name}/{}", file.to_str().unwrap()), out);
            }
        } else {
            out.push(name.to_owned());
        }
    }

    fn script_comment_lines(name: &str, text: &str) -> Result<Vec<usize>, String> {
        let shebang = text.lines().next().filter(|l| l.starts_with("#!"));
        let ext = Path::new(name).extension().and_then(|e| e.to_str());
        let (prefix, python) = match (ext, shebang) {
            (Some("py"), _) => ("#", true),
            (Some("sh" | "yml" | "yaml" | "toml"), _) => ("#", false),
            (Some("lua"), _) => ("--", false),
            (None, Some(s)) => ("#", s.contains("python")),
            _ => return Ok(Vec::new()),
        };
        let mut quote: Option<(&str, usize)> = None;
        let mut lines = Vec::new();
        for (i, line) in text.lines().enumerate() {
            if quote.is_none()
                && line.trim_start().starts_with(prefix)
                && !(i == 0 && shebang.is_some())
            {
                lines.push(i + 1);
                continue;
            }
            if !python {
                continue;
            }
            let mut rest = line;
            loop {
                if let Some((q, _)) = quote {
                    let Some((_, after)) = rest.split_once(q) else {
                        break;
                    };
                    quote = None;
                    rest = after;
                    continue;
                }
                let Some(tail) = rest.find(['#', '"', '\'']).and_then(|at| rest.get(at..)) else {
                    break;
                };
                if tail.starts_with('#') {
                    break;
                }
                if let Some((q, after)) = ["\"\"\"", "'''"]
                    .into_iter()
                    .find_map(|q| Some((q, tail.strip_prefix(q)?)))
                {
                    quote = Some((q, i + 1));
                    rest = after;
                    continue;
                }
                let q = tail.as_bytes()[0];
                let mut end = None;
                let mut j = 1;
                while j < tail.len() {
                    match tail.as_bytes()[j] {
                        b'\\' => j += 1,
                        b if b == q => {
                            end = Some(j);
                            break;
                        }
                        _ => {}
                    }
                    j += 1;
                }
                let Some(end) = end else {
                    return Err(format!("line {}: a string the check cannot close", i + 1));
                };
                rest = tail.get(end + 1..).unwrap();
            }
        }
        match quote {
            Some((_, from)) => Err(format!("line {from}: a string that never closes")),
            None => Ok(lines),
        }
    }

    fn script_comments(root: &Path, name: &str) -> Result<(String, Vec<usize>), String> {
        let text = std::fs::read_to_string(root.join(name)).map_err(|e| format!("{name}: {e}"))?;
        let found = script_comment_lines(name, &text).map_err(|e| format!("{name}: {e}"))?;
        Ok((text, found))
    }

    #[test]
    fn script_comment_lines_skip_shebangs_and_python_strings() {
        let py = "#!/usr/bin/env python3\n\"\"\"Usage:\n# no\n\"\"\"\nx = '''a''' # no\n  # one\ny = \"\"\"\n# no\n'''\n\"\"\"\n# two\n";
        assert_eq!(script_comment_lines("a.py", py).unwrap(), [6, 11]);
        assert_eq!(script_comment_lines("tools/run", py).unwrap(), [6, 11]);
        let sh = "#!/bin/sh\n# one\necho '# no'\n  # two\n";
        assert_eq!(script_comment_lines("tools/x", sh).unwrap(), [2, 4]);
        assert_eq!(
            script_comment_lines("a.yml", "a: 1 # no\n# one\n").unwrap(),
            [2]
        );
        assert_eq!(
            script_comment_lines("a.lua", "-- one\nx = 1 -- no\n").unwrap(),
            [1]
        );
        assert_eq!(
            script_comment_lines("a.tsv", "# no\n").unwrap(),
            [] as [usize; 0]
        );
    }

    #[test]
    fn script_comment_lines_see_past_a_triple_quote_in_a_one_line_string() {
        let py = r#"a = line.startswith('"""', i) or b == "'''"
# one
c = "\"" + '\''
# two
d = """
# no
"""
# three
"#;
        assert_eq!(script_comment_lines("a.py", py).unwrap(), [2, 4, 8]);
        assert_eq!(
            script_comment_lines("a.py", "x = 1  # a \"\"\" here\n# one\n").unwrap(),
            [2]
        );
    }

    #[test]
    fn the_scripts_check_fails_on_what_it_cannot_read() {
        assert!(script_comment_lines("a.py", "x = \"\"\"\n# no\n").is_err());
        assert!(script_comment_lines("a.py", "x = 'open\n# no\n").is_err());
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        script_files(root, "tools/no-such-script", &mut files);
        assert!(script_comments(root, &files[0]).is_err());
    }

    #[test]
    fn scripts_and_config_hold_no_comment_but_the_kept_ones() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        for name in SCRIPTS {
            script_files(root, name, &mut files);
        }
        files.sort();
        let mut wrong = Vec::new();
        for name in &files {
            let (text, mut found) = match script_comments(root, name) {
                Ok(read) => read,
                Err(e) => {
                    wrong.push(format!("cannot read {e}"));
                    continue;
                }
            };
            let lines: Vec<&str> = text.lines().collect();
            for &(_, first, _) in KEPT_COMMENTS.iter().filter(|(f, ..)| f == name) {
                let Some(at) = lines.iter().position(|l| l.trim() == first) else {
                    wrong.push(format!(
                        "{name}: `{first}` is gone: take it off KEPT_COMMENTS"
                    ));
                    continue;
                };
                if first.starts_with("\"\"\"") && !text.contains("__doc__") {
                    wrong.push(format!(
                        "{name}: its docstring is no longer read as `__doc__`"
                    ));
                }
                let mut line = at + 1;
                while let Some(i) = found.iter().position(|&l| l == line) {
                    found.remove(i);
                    line += 1;
                }
            }
            wrong.extend(
                found
                    .iter()
                    .map(|l| format!("{name}:{l}: {}", lines[l - 1].trim())),
            );
        }
        for (f, first, _) in KEPT_COMMENTS {
            if !files.iter().any(|n| n == f) {
                wrong.push(format!("{f}: `{first}` is gone: take it off KEPT_COMMENTS"));
            }
        }
        assert!(
            wrong.is_empty(),
            "code carries no comments; move the why into the commit message and the what into a \
             name or a test, or list a comment the code cannot do without in KEPT_COMMENTS with \
             its reason:\n{}",
            wrong.join("\n")
        );
    }
}
