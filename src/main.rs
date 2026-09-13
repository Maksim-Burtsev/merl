//! merl — a keyboard-only terminal code navigator.

mod app;
mod buffer;
mod picker;
mod search;
mod theme;
mod tree;
mod tutor;
mod ui;
mod wrap;

use std::io::stdout;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::crossterm::event::{
    Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::{execute, terminal};

use crate::app::{App, Focus};
use crate::buffer::Buffer;
use crate::tutor::Tutor;

/// Everything the event loop wakes up for.
enum Msg {
    Key(ratatui::crossterm::event::KeyEvent),
    Resize,
    /// nucleo found new matches.
    Redraw,
    /// Something changed in the directory of the open file.
    Fs(notify::Event),
}

#[derive(Parser)]
#[command(name = "merl", version, about = "Keyboard-only code navigator for the terminal")]
struct Cli {
    /// Directory, file, or file:LINE to open (default: the current directory)
    target: Option<String>,
    /// Colour theme
    #[arg(long, value_name = "NAME")]
    theme: Option<String>,
    /// Walk through every key on a bundled sample project (ignores the target)
    #[arg(long)]
    tutor: bool,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("merl: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let name = match &cli.theme {
        Some(n) => n.clone(),
        None => theme::config()?.theme,
    };
    let theme = theme::load(&name)?;

    let (root, file, line) = if cli.tutor {
        (tutor::extract()?, None, None)
    } else {
        resolve(cli.target.as_deref())?
    };
    let buf = match &file {
        Some(p) => Buffer::load(p)?,
        None => Buffer::empty(),
    };
    let (tree, files) = tree::build(&root);
    let dir = root.clone();
    let mut app = App::new(root, tree, files, buf, line);
    if cli.tutor {
        app.show_tree = false;
        app.focus = Focus::Code;
        app.tutor = Some(Tutor { step: 0, dir });
    }

    let mut terminal = ratatui::try_init()?;
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

    let result = event_loop(&mut terminal, &mut app, &theme, &rx, fs);

    if enhanced {
        let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    // Runs even when the loop returned an error: the sample project is ours to clean up.
    if let Some(t) = &app.tutor {
        let _ = std::fs::remove_dir_all(&t.dir);
    }
    result
}

fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    theme: &theme::Theme,
    rx: &mpsc::Receiver<Msg>,
    fs: Sender<Msg>,
) -> Result<()> {
    // One watcher for the whole run, following the open file's directory. A failing watcher
    // (too many open files, an unsupported filesystem) only costs auto-reload.
    let mut watcher = notify::recommended_watcher(move |ev: notify::Result<notify::Event>| {
        if let Ok(ev) = ev {
            let _ = fs.send(Msg::Fs(ev));
        }
    })
    .ok();
    let mut watched: Option<PathBuf> = None;

    let mut dirty = true;
    loop {
        // nucleo matches in the background; poll it while its overlay is on screen.
        if let Some(p) = &mut app.picker {
            dirty |= p.tick();
        }
        rewatch(watcher.as_mut(), &mut watched, app);
        if dirty {
            terminal.draw(|f| ui::draw(f, app, theme))?;
            dirty = false;
        }
        let idle = if app.picker.is_some() { 10 } else { 100 };
        match rx.recv_timeout(Duration::from_millis(idle)) {
            Ok(Msg::Key(k)) => {
                if app.key(k) {
                    return Ok(());
                }
                dirty = true;
            }
            Ok(Msg::Resize) | Ok(Msg::Redraw) => dirty = true,
            Ok(Msg::Fs(ev)) => {
                if concerns_open_file(app, &ev) {
                    app.reload();
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

fn git_toplevel(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|d| d.join(".git").exists())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
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
