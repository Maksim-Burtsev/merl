//! merl — a keyboard-only terminal code navigator.

mod app;
mod buffer;
mod git;
mod line_edit;
mod live;
mod picker;
mod search;
mod theme;
mod tree;
mod tutor;
mod ui;
mod wrap;

use std::collections::HashSet;
use std::io::{Write, stdout};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

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
use crate::picker::PickItem;
use crate::tutor::Tutor;

/// Everything the event loop wakes up for.
enum Msg {
    Key(ratatui::crossterm::event::KeyEvent),
    /// The terminal pasted text (bracketed paste).
    Paste(String),
    Resize,
    /// Something changed in the directory of the open file, or anywhere in the project.
    Fs(notify::Event),
    /// The project was walked again after it changed on disk.
    Project(tree::Tree, Vec<PathBuf>),
    /// The branch under review was listed again, or why git could not.
    Review(Result<git::Review, String>),
    /// `git diff` finished for the file at this path.
    Diff(PathBuf, git::Diff),
    /// The grep with this number finished: the rows it found.
    Search(u64, Vec<PickItem>),
    /// SIGTERM, SIGHUP or SIGINT from outside: save and leave as `q` does.
    Quit,
}

#[derive(Parser)]
#[command(
    name = "merl",
    version,
    about = "Keyboard-only code navigator for the terminal"
)]
struct Cli {
    /// Directory, file, or FILE:LINE[:COL] to open (default: the current directory)
    target: Option<String>,
    /// Colour theme
    #[arg(long, value_name = "NAME")]
    theme: Option<String>,
    /// Walk through every key on a bundled sample project (ignores the target)
    #[arg(long)]
    tutor: bool,
    /// Review the checked-out branch (or `--review=BRANCH` to switch to it first): its files
    /// in the panel, its diff over the code, c / C between hunks
    #[arg(
        long,
        value_name = "BRANCH",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = ""
    )]
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

    let (mut root, shallow, mut file, line) = if cli.tutor {
        (tutor::extract()?, false, None, None)
    } else {
        resolve(cli.target.as_deref())?
    };
    let review = match &cli.review {
        Some(branch) => {
            root = git_toplevel(&root).context("--review needs a git repository")?;
            let branch = Some(branch.as_str()).filter(|b| !b.is_empty());
            let r = git::Review::open(&root, branch, cli.base.as_deref())?;
            if file.is_none() {
                // The first file with something to read; a branch of binaries opens on one.
                let on_disk = || r.files.iter().filter(|f| f.status != 'D');
                file = on_disk()
                    .find(|f| f.has_hunks())
                    .or_else(|| on_disk().next())
                    .map(|f| root.join(&f.path));
            }
            Some(r)
        }
        None => None,
    };
    let (mut tree, files) = tree::build(&root, shallow);
    // Of the walk, before the review panel takes the tree's place.
    let project = live::Project::new(&root, shallow, &tree);
    // `o` offers them in a review too, and the panel has none.
    let ignored = tree.ignored_files();
    if let Some(r) = &review {
        tree = tree::from_files(&r.files.iter().map(|f| f.path.clone()).collect::<Vec<_>>());
    }
    let buf = match &file {
        Some(p) => Buffer::load(p)?,
        None => Buffer::empty(),
    };
    let dir = root.clone();
    let mut app = App::new(root, tree, files, buf, line);
    app.shallow = shallow;
    app.ignored = ignored;
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
    // Without this a `kill` or a closed tmux pane skips the cleanup below: the terminal stays on
    // the alternate screen and edits younger than the autosave delay are lost.
    let quit = tx.clone();
    if let Ok(mut signals) = signal_hook::iterator::Signals::new([
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGHUP,
        signal_hook::consts::SIGINT,
    ]) {
        std::thread::spawn(move || {
            if signals.forever().next().is_some() {
                let _ = quit.send(Msg::Quit);
            }
        });
    }
    let result = event_loop(&mut terminal, &mut app, theme, &rx, tx, project);

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
    mut project: live::Project,
) -> Result<()> {
    let diff_tx = fs.clone();
    let project_fs = fs.clone();
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
    // A second one for the project: the open file may be outside it, and its directory comes
    // and goes.
    let mut project_watcher =
        notify::recommended_watcher(move |ev: notify::Result<notify::Event>| {
            if let Ok(ev) = ev {
                let _ = project_fs.send(Msg::Fs(ev));
            }
        })
        .ok();
    let mut project_watched = HashSet::new();
    if let Some(w) = &mut project_watcher {
        project.watch(w, &mut project_watched);
    }
    // The review panel follows the same events, and the branch inside `.git`.
    let mut review = (app.review.as_ref())
        .and_then(|_| git::dirs(&app.root))
        .map(|(git_dir, common_dir)| live::Review::new(&git_dir, &common_dir));
    if let (Some(r), Some(w)) = (&review, &mut project_watcher) {
        r.watch(w);
    }

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
                let diff = git::diff(&root, &path, None, None);
                let _ = tx.send(Msg::Diff(path, diff));
            });
        }
        if project.walk_due(Instant::now()) {
            // In a thread: the walk takes 0.1 s on 12k files.
            let (tx, root, shallow) = (diff_tx.clone(), app.root.clone(), app.shallow);
            std::thread::spawn(move || {
                let (tree, files) = tree::build(&root, shallow);
                let _ = tx.send(Msg::Project(tree, files));
            });
        }
        if let (Some(live), Some(r)) = (&mut review, &app.review)
            && live.list_due(Instant::now())
        {
            // In a thread, like the marks: four git commands over the whole branch.
            let (tx, root, r) = (diff_tx.clone(), app.root.clone(), r.clone());
            std::thread::spawn(move || {
                let fresh = r.refresh(&root).map_err(|e| format!("{e:#}"));
                let _ = tx.send(Msg::Review(fresh));
            });
        }
        if let Some(job) = app.search_tick() {
            // In a thread: a grep over a large project takes longer than a keystroke.
            let tx = diff_tx.clone();
            std::thread::spawn(move || {
                let _ = tx.send(Msg::Search(job.seq, job.items()));
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
            // A broken theme file of the user's own must not take merl down mid-preview: say so
            // and keep drawing the one already loaded.
            match theme::load(&loaded) {
                Ok(t) => {
                    theme = t;
                    app.buf.clear_hl();
                }
                Err(e) => app.message = format!("{e:#}"),
            }
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
                // An ignored directory expanded or collapsed in the tree is watched, or no more.
                if project.shown(&app.tree)
                    && let Some(w) = &mut project_watcher
                {
                    project.watch(w, &mut project_watched);
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
            Ok(Msg::Quit) => {
                // Nobody is there to answer a conflict; edits that cannot be saved are lost.
                app.flush();
                return Ok(());
            }
            Ok(Msg::Search(seq, items)) => dirty |= app.search_done(seq, items),
            Ok(Msg::Resize) => dirty = true,
            Ok(Msg::Fs(ev)) => {
                // A reload that found the file as merl knows it is not worth a frame: the
                // project watch reports every namesake under the root.
                dirty |= concerns_open_file(app, &ev) && app.reload(false);
                project.event(&ev, Instant::now());
                if let Some(r) = &mut review {
                    r.event(&ev, project.touched(&ev), Instant::now());
                }
            }
            Ok(Msg::Project(tree, files)) => {
                project.walked(&tree, Instant::now());
                app.project_walked(tree, files);
                // The tree read its open ignored directories again: they are listed as read.
                project.shown(&app.tree);
                if let Some(w) = &mut project_watcher {
                    project.watch(w, &mut project_watched);
                }
                if let Some(r) = &mut review {
                    r.touch(Instant::now());
                }
                dirty = true;
            }
            Ok(Msg::Review(fresh)) => {
                if let Some(r) = &mut review {
                    r.listed();
                }
                // What git could not list stays as it was until the next event, and says so:
                // a base that was deleted fails every time.
                dirty |= match fresh {
                    Ok(fresh) => app.review_refreshed(fresh),
                    Err(e) => {
                        app.message = format!("review not refreshed: {e}");
                        true
                    }
                };
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

/// Does this filesystem event touch the open file? The project watch reports the whole root,
/// so the directory counts too: an `index.js` in `node_modules/` is not the open `index.js`.
/// FSEvents reports canonical `/private/...` paths, so the directory is compared both ways.
fn concerns_open_file(app: &App, ev: &notify::Event) -> bool {
    if matches!(ev.kind, EventKind::Access(_)) {
        return false;
    }
    let Some(open) = app.buf.path.as_deref() else {
        return false;
    };
    let (name, dir) = (open.file_name(), open.parent());
    let mut real = None;
    ev.paths.iter().any(|p| {
        p.file_name() == name
            && (p.parent() == dir
                || p.parent()
                    == real
                        .get_or_insert_with(|| dir?.canonicalize().ok())
                        .as_deref())
    })
}

/// A 1-based line and column, as compilers print them.
type LineCol = (usize, usize);

/// Turns the CLI target into `(project root, shallow, file to open, where in it)`.
fn resolve(target: Option<&str>) -> Result<(PathBuf, bool, Option<PathBuf>, Option<LineCol>)> {
    let Some(target) = target else {
        return Ok((std::env::current_dir()?, false, None, None));
    };
    let (path, line) = split_line(target);
    let path = PathBuf::from(path);
    if path.is_dir() {
        return Ok((path.canonicalize()?, false, None, None));
    }
    if !path.exists() {
        anyhow::bail!("{}: no such file or directory", path.display());
    }
    let path = path
        .canonicalize()
        .with_context(|| format!("{}", path.display()))?;
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    // Outside a repository nothing names a project, and the directory of `~/.zshrc` is the
    // whole home: the project is the files next to this one (#182).
    let (root, shallow) = git_toplevel(&dir).map_or((dir, true), |top| (top, false));
    Ok((root, shallow, Some(path), line))
}

/// Splits `FILE:LINE[:COL]`, as compilers print it, into the path and `(line, column)`. The path
/// is the longest prefix before a colon that exists, so the rest of a grep line, `FILE:LINE:text`,
/// is ignored, and a file really called `a:12` opens as itself. No column is column 1.
fn split_line(target: &str) -> (&str, Option<LineCol>) {
    if Path::new(target).exists() {
        return (target, None);
    }
    let found = target
        .rmatch_indices(':')
        .map(|(i, _)| (&target[..i], &target[i + 1..]))
        .find(|(path, _)| Path::new(path).exists());
    let Some((path, rest)) = found else {
        return (target, None);
    };
    let mut nums = rest.split(':').map(|n| n.parse().ok());
    let line = nums.next().flatten();
    let col = nums.next().flatten().unwrap_or(1);
    (path, line.map(|n| (n, col)))
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
    fn review_takes_its_branch_only_with_an_equals_sign() {
        use clap::Parser;
        let cli = super::Cli::parse_from(["merl", "--review", "src/main.rs"]);
        assert_eq!(
            (cli.review.as_deref(), cli.target.as_deref()),
            (Some(""), Some("src/main.rs"))
        );
        let cli = super::Cli::parse_from(["merl", "--review=feature", "--base", "origin/dev"]);
        assert_eq!(
            (cli.review.as_deref(), cli.base.as_deref()),
            (Some("feature"), Some("origin/dev"))
        );
        assert!(super::Cli::try_parse_from(["merl", "--base", "x"]).is_err());
    }

    #[test]
    fn a_namesake_elsewhere_in_the_project_is_not_the_open_file() {
        use notify::{Event, EventKind, event::CreateKind};
        let dir = std::env::temp_dir().join(format!("merl-namesake-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("node_modules/pkg")).unwrap();
        std::fs::write(dir.join("index.js"), "x\n").unwrap();
        let buf = crate::buffer::Buffer::load(&dir.join("index.js")).unwrap();
        let app = crate::app::App::new(dir.clone(), Default::default(), Vec::new(), buf, None);
        let real = dir.canonicalize().unwrap();
        for (path, want) in [
            (dir.join("index.js"), true),
            (real.join("index.js"), true), // as FSEvents spells it
            (real.join("node_modules/pkg/index.js"), false),
            (real.join("main.js"), false),
        ] {
            let ev = Event::new(EventKind::Create(CreateKind::File)).add_path(path.clone());
            assert_eq!(super::concerns_open_file(&app, &ev), want, "{path:?}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_file_outside_a_repository_lists_its_neighbours_not_all_below_them() {
        // #182: `merl ~/.zshrc` walked the whole home directory before the first frame.
        let dir = std::env::temp_dir().join(format!("merl-no-repo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("deep/er")).unwrap();
        for f in ["a.txt", "b.txt", "deep/c.txt", "deep/er/d.txt"] {
            std::fs::write(dir.join(f), "x\n").unwrap();
        }
        let real = dir.canonicalize().unwrap();
        let walk = |target: &Path| {
            let (root, shallow, file, _) = super::resolve(target.to_str()).unwrap();
            assert_eq!(root, real);
            (file, crate::tree::build(&root, shallow).1)
        };
        let (file, files) = walk(&dir.join("a.txt"));
        assert_eq!(file, Some(real.join("a.txt")));
        assert_eq!(files, [Path::new("a.txt"), Path::new("b.txt")]);
        // Named, the directory is a project: walked all the way down.
        assert_eq!(walk(&dir).1.len(), 4);
        // In a repository, the repository.
        std::fs::create_dir(dir.join(".git")).unwrap();
        assert_eq!(walk(&dir.join("deep/c.txt")).1.len(), 4);
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
        // A grep line, and ripgrep's `--column` one, whose text has colons of its own.
        assert_eq!(split_line("src/main.rs:12:mod app;"), (main, Some((12, 1))));
        assert_eq!(
            split_line("src/main.rs:16:5:use std::collections::HashSet;"),
            (main, Some((16, 5)))
        );
        assert_eq!(split_line("nope:12"), ("nope:12", None));
        assert_eq!(split_line("nope.rs:12:5"), ("nope.rs:12:5", None));

        // A colon in a name that really exists is part of the name.
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
}
