//! merl — a keyboard-only terminal code navigator.

mod app;
mod buffer;
mod git;
mod picker;
mod search;
mod theme;
mod tree;
mod tutor;
mod ui;
mod wrap;

use std::io::{Write, stdout};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::crossterm::cursor::{SetCursorStyle, Show};
use ratatui::crossterm::event::{
    DisableBracketedPaste, EnableBracketedPaste, Event, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::{execute, terminal};

use crate::app::{App, Focus, Mode};
use crate::buffer::Buffer;
use crate::tutor::Tutor;

/// Everything the event loop wakes up for.
enum Msg {
    Key(ratatui::crossterm::event::KeyEvent),
    /// The terminal pasted text (bracketed paste).
    Paste(String),
    Resize,
    /// nucleo found new matches.
    Redraw,
    /// Something changed in the directory of the open file.
    Fs(notify::Event),
    /// `git diff` finished for the file at this path.
    Diff(PathBuf, git::Diff),
}

#[derive(Parser)]
#[command(
    name = "merl",
    version,
    about = "Keyboard-only code navigator for the terminal"
)]
struct Cli {
    /// Directory, file, or file:LINE to open (default: the current directory)
    target: Option<String>,
    /// Colour theme
    #[arg(long, value_name = "NAME")]
    theme: Option<String>,
    /// Walk through every key on a bundled sample project (ignores the target)
    #[arg(long)]
    tutor: bool,
    /// Review the checked-out branch (or switch to BRANCH first): its files in the panel,
    /// its diff over the code, c / C between hunks
    #[arg(long, value_name = "BRANCH", num_args = 0..=1, default_missing_value = "")]
    review: Option<String>,
    /// The branch the review is against (default: origin/HEAD, then origin/master, main, develop)
    #[arg(long, value_name = "REF", requires = "review")]
    base: Option<String>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("merl: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = theme::config()?;
    let name = cli.theme.clone().unwrap_or(config.theme);
    let theme = theme::load(&name)?;

    let (mut root, mut file, line) = if cli.tutor {
        (tutor::extract()?, None, None)
    } else {
        resolve(cli.target.as_deref())?
    };
    let review = match &cli.review {
        Some(branch) => {
            root = git_toplevel(&root).context("--review needs a git repository")?;
            let branch = Some(branch.as_str()).filter(|b| !b.is_empty());
            let r = git::Review::open(&root, branch, cli.base.as_deref())?;
            if file.is_none() {
                file = r
                    .files
                    .iter()
                    .find(|f| f.status != 'D')
                    .map(|f| root.join(&f.path));
            }
            Some(r)
        }
        None => None,
    };
    let (mut tree, files) = tree::build(&root);
    if let Some(r) = &review {
        tree = tree::from_files(&r.files.iter().map(|f| f.path.clone()).collect::<Vec<_>>());
    }
    let buf = match &file {
        Some(p) => Buffer::load(p)?,
        None => Buffer::empty(),
    };
    let dir = root.clone();
    let mut app = App::new(root, tree, files, buf, line);
    if let Some(r) = review {
        app.start_review(r);
    }
    app.autosave = Duration::from_millis(config.autosave_delay_ms);
    app.theme = name;
    app.config = theme::config_path();
    if cli.tutor {
        app.show_tree = false;
        app.focus = Focus::Code;
        app.tutor = Some(Tutor { step: 0, dir });
    }

    let mut terminal = ratatui::try_init()?;
    let _ = execute!(stdout(), EnableBracketedPaste);
    let enhanced = terminal::supports_keyboard_enhancement().unwrap_or(false);
    if enhanced {
        execute!(
            stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
            )
        )?;
        // ratatui::try_init already installed a restoring hook; pop our flags before it runs.
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
            hook(info);
        }));
    }

    // The key reader must start after the enhancement query, which blocks on a pending read.
    let (tx, rx) = mpsc::channel();
    let keys: Sender<Msg> = tx.clone();
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
    let fs = tx.clone();
    app.wake = std::sync::Arc::new(move || {
        let _ = tx.send(Msg::Redraw);
    });

    let result = event_loop(&mut terminal, &mut app, theme, &rx, fs);

    if enhanced {
        let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    }
    let _ = execute!(
        stdout(),
        SetCursorStyle::DefaultUserShape,
        DisableBracketedPaste
    );
    ratatui::restore();
    // The last frame may have hidden the cursor (welcome screen, `?` overlay); leaving the
    // alternate screen does not bring it back in xterm-like terminals such as Ghostty.
    let _ = execute!(stdout(), Show);
    // Runs even when the loop returned an error: the sample project is ours to clean up.
    if let Some(t) = &app.tutor {
        let _ = std::fs::remove_dir_all(&t.dir);
    }
    result
}

fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    mut theme: theme::Theme,
    rx: &mpsc::Receiver<Msg>,
    fs: Sender<Msg>,
) -> Result<()> {
    let diff_tx = fs.clone();
    let mut loaded = app.theme.clone();
    // One watcher for the whole run, following the open file's directory. A failing watcher
    // (too many open files, an unsupported filesystem) only costs auto-reload.
    let mut watcher = notify::recommended_watcher(move |ev: notify::Result<notify::Event>| {
        if let Ok(ev) = ev {
            let _ = fs.send(Msg::Fs(ev));
        }
    })
    .ok();
    app.no_watch = watcher.is_none();
    let mut watched: Option<PathBuf> = None;

    let mut dirty = true;
    let mut typing: Option<bool> = None;
    loop {
        // nucleo matches in the background; poll it while its overlay is on screen.
        if let Some(p) = &mut app.picker {
            dirty |= p.tick();
        }
        dirty |= app.tick();
        rewatch(watcher.as_mut(), &mut watched, app);
        if std::mem::take(&mut app.want_diff)
            && let Some(path) = app.buf.path.clone()
        {
            // In a thread: git on a large repository can take longer than a frame.
            let (tx, root) = (diff_tx.clone(), app.root.clone());
            std::thread::spawn(move || {
                let diff = git::diff(&root, &path, None);
                let _ = tx.send(Msg::Diff(path, diff));
            });
        }
        // Typing (edit mode, or any prompt) gets a bar, navigating a block, like vim: the shape
        // says which mode you are in without looking at the status bar.
        let now = !matches!(app.mode, Mode::Normal | Mode::Help);
        if typing != Some(now) {
            typing = Some(now);
            let shape = if now {
                SetCursorStyle::SteadyBar
            } else {
                SetCursorStyle::SteadyBlock
            };
            let _ = execute!(stdout(), shape);
        }
        // The theme picker previews by moving what `shown_theme` names, after a key or after
        // nucleo re-sorted the list under the cursor.
        if app.shown_theme() != loaded {
            loaded = app.shown_theme().to_string();
            theme = theme::load(&loaded)?;
            app.buf.clear_hl();
            dirty = true;
        }
        if dirty {
            terminal.draw(|f| ui::draw(f, app, &theme))?;
            dirty = false;
        }
        let idle = if app.picker.is_some() { 10 } else { 100 };
        match rx.recv_timeout(Duration::from_millis(idle)) {
            Ok(Msg::Key(k)) => {
                if app.key(k) {
                    return Ok(());
                }
                if let Some(text) = app.clipboard.take() {
                    // OSC 52: the terminal puts it on the system clipboard, even over ssh.
                    let mut out = stdout();
                    let _ = write!(out, "\x1b]52;c;{}\x07", base64(text.as_bytes()));
                    let _ = out.flush();
                }
                dirty = true;
            }
            Ok(Msg::Paste(text)) => {
                app.paste(&text);
                dirty = true;
            }
            Ok(Msg::Diff(path, diff)) => {
                if app.buf.path == Some(path) && app.review.is_none() {
                    app.diff = diff;
                    dirty = true;
                }
            }
            Ok(Msg::Resize) | Ok(Msg::Redraw) => dirty = true,
            Ok(Msg::Fs(ev)) => {
                if concerns_open_file(app, &ev) {
                    app.reload(false);
                    dirty = true;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}

/// Watches the parent directory of the open file, non-recursively: editors save by writing a
/// temporary file and renaming it over the original, which a watch on the file itself misses.
fn rewatch(watcher: Option<&mut RecommendedWatcher>, watched: &mut Option<PathBuf>, app: &App) {
    let dir = app.buf.path.as_deref().and_then(Path::parent);
    if dir == watched.as_deref() {
        return;
    }
    let Some(watcher) = watcher else { return };
    if let Some(old) = watched.take() {
        let _ = watcher.unwatch(&old);
    }
    if let Some(dir) = dir {
        // Recorded even when the watch fails, so a directory that cannot be watched is not
        // retried on every pass of the event loop.
        let _ = watcher.watch(dir, RecursiveMode::NonRecursive);
        *watched = Some(dir.to_path_buf());
    }
}

/// Does this filesystem event touch the open file? FSEvents reports canonical `/private/...`
/// paths, so only the file name is comparable.
fn concerns_open_file(app: &App, ev: &notify::Event) -> bool {
    if matches!(ev.kind, EventKind::Access(_)) {
        return false;
    }
    let Some(name) = app.buf.path.as_deref().and_then(Path::file_name) else {
        return false;
    };
    ev.paths.iter().any(|p| p.file_name() == Some(name))
}

/// Turns the CLI target into `(project root, file to open, 1-based line)`.
fn resolve(target: Option<&str>) -> Result<(PathBuf, Option<PathBuf>, Option<usize>)> {
    let Some(target) = target else {
        return Ok((std::env::current_dir()?, None, None));
    };
    let (path, line) = split_line(target);
    let path = PathBuf::from(path);
    if path.is_dir() {
        return Ok((path.canonicalize()?, None, None));
    }
    if !path.exists() {
        anyhow::bail!("{}: no such file or directory", path.display());
    }
    let path = path
        .canonicalize()
        .with_context(|| format!("{}", path.display()))?;
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let root = git_toplevel(&dir).unwrap_or(dir);
    Ok((root, Some(path), line))
}

/// Splits a trailing `:LINE`, but only when the prefix is a path that exists.
fn split_line(target: &str) -> (&str, Option<usize>) {
    if Path::new(target).exists() {
        return (target, None);
    }
    match target.rsplit_once(':') {
        Some((path, n)) if Path::new(path).exists() => (path, n.parse().ok()),
        _ => (target, None),
    }
}

/// Standard base64 with padding; the stdlib has none and a dependency is more than this.
fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (*b as u32) << (16 - 8 * i));
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

fn git_toplevel(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|d| d.join(".git").exists())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_the_standard_alphabet_and_padding() {
        assert_eq!(super::base64(b""), "");
        assert_eq!(super::base64(b"f"), "Zg==");
        assert_eq!(super::base64(b"fo"), "Zm8=");
        assert_eq!(super::base64(b"foo"), "Zm9v");
        assert_eq!(super::base64("hi\nтам".as_bytes()), "aGkK0YLQsNC8");
    }

    #[test]
    fn splits_only_on_existing_paths() {
        assert_eq!(
            super::split_line("src/main.rs:20"),
            ("src/main.rs", Some(20))
        );
        assert_eq!(super::split_line("src/main.rs"), ("src/main.rs", None));
        // A colon in a name that really exists must not be treated as a line number.
        assert_eq!(super::split_line("nope:12"), ("nope:12", None));
    }
}
