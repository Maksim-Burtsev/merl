//! merl — a read-only terminal code navigator.

mod app;
mod buffer;
mod theme;
mod ui;
mod wrap;

use std::io::stdout;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use ratatui::crossterm::event::{
    Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::{execute, terminal};

use crate::app::App;
use crate::buffer::Buffer;

#[derive(Parser)]
#[command(name = "merl", version, about = "Read-only terminal code navigator")]
struct Cli {
    /// Directory, file, or file:LINE to open (default: the current directory)
    target: Option<String>,
    /// Colour theme
    #[arg(long, value_name = "NAME")]
    theme: Option<String>,
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

    let (root, file, line) = resolve(cli.target.as_deref())?;
    let buf = match &file {
        Some(p) => Buffer::load(p)?,
        None => Buffer::empty(),
    };
    let mut app = App::new(root, buf, line);

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
    std::thread::spawn(move || {
        while let Ok(ev) = ratatui::crossterm::event::read() {
            if tx.send(ev).is_err() {
                break;
            }
        }
    });

    let result = event_loop(&mut terminal, &mut app, &theme, &rx);

    if enhanced {
        let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}

fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    theme: &theme::Theme,
    rx: &mpsc::Receiver<Event>,
) -> Result<()> {
    let mut dirty = true;
    loop {
        if dirty {
            terminal.draw(|f| ui::draw(f, app, theme))?;
            dirty = false;
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Event::Key(k)) => {
                if app.key(k) {
                    return Ok(());
                }
                dirty = true;
            }
            Ok(Event::Resize(..)) => dirty = true,
            Ok(_) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
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
