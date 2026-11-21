use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::JoinHandle;

use super::{
    App, Arc, Buffer, CMode, Duration, HashMap, Instant, KeyCode, KeyEvent, KeyModifiers, Kind,
    Mode, PathBuf, Paths, PickItem, PickerKind, TextLine, Tree, Walked, buffer, git, search,
};

pub const SEARCHING_AFTER: Duration = Duration::from_millis(250);
pub(super) const SEARCHING: &str = "searching…";

type Warming = HashMap<Kind, Option<JoinHandle<Walked>>>;
type AtBase = Option<(String, Box<App>)>;

#[derive(Default)]
pub struct Background {
    pub waits: Option<Duration>,
    searcher: Option<Searcher>,
    search: Option<Search>,
}

struct Searcher {
    asks: Sender<Ask>,
    told: Receiver<Told>,
    dropped: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

struct Search {
    place: Place,
    since: Instant,
    told: Option<Told>,
}

#[derive(PartialEq)]
struct Place {
    at: Option<(PathBuf, TextLine, usize)>,
    edits: usize,
    on_disk: u64,
    previewing: bool,
}

#[derive(Clone)]
struct Caches {
    external: HashMap<Kind, (Vec<PathBuf>, Paths)>,
    walked_roots: HashMap<PathBuf, Paths>,
    node_modules_of: Option<PathBuf>,
    otp: Option<Vec<PathBuf>>,
    c_includes: HashMap<(PathBuf, CMode), Paths>,
    clang_include: std::sync::OnceLock<Option<PathBuf>>,
    c_files: HashMap<CMode, (Paths, Paths)>,
}

struct Ask {
    root: PathBuf,
    files: Vec<PathBuf>,
    path: Option<PathBuf>,
    lines: Vec<String>,
    format: buffer::Format,
    at: (usize, usize, Option<(usize, usize)>),
    review: Option<git::Review>,
    diff: git::Diff,
    go_build: search::GoBuild,
    shaped: Arc<search::ShapedFiles>,
    caches: Caches,
    warming: Warming,
}

struct Told {
    message: String,
    message_path: Option<(String, usize)>,
    picked: Option<(PickerKind, String, Vec<PickItem>)>,
    jumped: Vec<PickItem>,
    caches: Caches,
    warming: Warming,
}

impl Searcher {
    fn start() -> Self {
        let (asks, asked) = mpsc::channel::<Ask>();
        let (tell, told) = mpsc::channel();
        let dropped = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&dropped);
        let thread = std::thread::spawn(move || {
            search::drop_this_thread_s_search_on(flag);
            let mut at_base = None;
            for ask in asked {
                let told = ask.answer(&mut at_base);
                if tell.send(told).is_err() {
                    break;
                }
            }
        });
        Self {
            asks,
            told,
            dropped,
            thread,
        }
    }
}

impl Ask {
    fn answer(self, at_base: &mut AtBase) -> Told {
        let mut buf = Buffer::empty();
        (buf.path, buf.lines) = (self.path, self.lines);
        buf.set_format(self.format);
        let mut app = App::new(self.root, Tree::default(), self.files, buf, None);
        (app.line, app.col, app.deleted) = self.at;
        (app.review, app.diff, app.go_build) = (self.review, self.diff, self.go_build);
        (app.shaped, app.warming) = (self.shaped, self.warming);
        app.adopt(self.caches);
        app.at_base = at_base.take();
        app.history.clear();
        let (message, picked, _) = app.answer_of_d();
        *at_base = app.at_base.take();
        let last = app.history.len().saturating_sub(1);
        let jumped = (app.history.iter().enumerate())
            .filter_map(|(n, (path, at, col))| {
                let (line, deleted) = match *at {
                    TextLine::File(l) => (l + 1, false),
                    TextLine::Deleted(k, i) if n == last => {
                        let from = app.diff.ghost_from.get(&k).copied().unwrap_or(0);
                        (from + i + 1, true)
                    }
                    TextLine::Deleted(..) => return None,
                };
                Some(PickItem {
                    label: String::new(),
                    path: path.clone(),
                    line,
                    col: *col,
                    code_at: None,
                    path_at: None,
                    deleted,
                })
            })
            .collect();
        Told {
            message,
            message_path: app.message_path.take(),
            picked,
            jumped,
            caches: app.caches(),
            warming: std::mem::take(&mut app.warming),
        }
    }
}

impl App {
    pub fn searching(&self) -> bool {
        self.background.search.is_some()
    }

    fn place(&self) -> Place {
        Place {
            at: self.pos(),
            edits: self.undo.len(),
            on_disk: self.buf.disk_hash,
            previewing: self.previewing(),
        }
    }

    fn caches(&self) -> Caches {
        Caches {
            external: self.external.clone(),
            walked_roots: self.walked_roots.clone(),
            node_modules_of: self.node_modules_of.clone(),
            otp: self.otp.clone(),
            c_includes: self.c_includes.clone(),
            clang_include: self.clang_include.clone(),
            c_files: self.c_files.clone(),
        }
    }

    fn adopt(&mut self, c: Caches) {
        (self.external, self.walked_roots) = (c.external, c.walked_roots);
        (self.node_modules_of, self.otp) = (c.node_modules_of, c.otp);
        (self.c_includes, self.clang_include, self.c_files) =
            (c.c_includes, c.clang_include, c.c_files);
    }

    pub(super) fn define(&mut self) {
        let Some(wait) = self.background.waits else {
            return self.goto_definition();
        };
        let place = self.place();
        let search = match self.background.search.take() {
            Some(s) if s.place == place => s,
            stale => {
                self.background.search = stale;
                self.drop_search();
                if !self.flush() {
                    return self.goto_definition();
                }
                self.ask(self.place())
            }
        };
        self.wait_for(search, wait);
    }

    fn ask(&mut self, place: Place) -> Search {
        let warming = (self.warming.iter_mut())
            .filter_map(|(kind, walk)| walk.take().map(|w| (*kind, Some(w))))
            .collect();
        let ask = Ask {
            root: self.root.clone(),
            files: self.files.clone(),
            path: self.buf.path.clone(),
            lines: self.buf.lines.clone(),
            format: self.buf.format(),
            at: (self.line, self.col, self.deleted),
            review: self.review.clone(),
            diff: self.diff.clone(),
            go_build: self.go_build.clone(),
            shaped: Arc::clone(&self.shaped),
            caches: self.caches(),
            warming,
        };
        let searcher = self.background.searcher.get_or_insert_with(Searcher::start);
        if searcher.asks.send(ask).is_err() {
            self.searcher_died();
        }
        Search {
            place,
            since: Instant::now(),
            told: None,
        }
    }

    fn wait_for(&mut self, mut search: Search, wait: Duration) {
        if search.told.is_none() {
            let Some(searcher) = &self.background.searcher else {
                return;
            };
            match searcher
                .told
                .recv_timeout(wait.saturating_sub(search.since.elapsed()))
            {
                Ok(told) => search.told = Some(told),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return self.searcher_died(),
            }
        }
        if let Some(told) = search.told {
            self.answered(told);
        } else {
            self.background.search = Some(search);
            self.message = SEARCHING.into();
            self.keys_action_of_key_in_hand = None;
        }
    }

    fn answered(&mut self, told: Told) {
        self.adopt(told.caches);
        self.warming.extend(told.warming);
        for to in told.jumped {
            self.jump_to_item(&to);
            let refused = self.buf.path.as_deref() != Some(to.path.as_path());
            if refused {
                return;
            }
        }
        if let Some((kind, title, items)) = told.picked {
            self.show_picker(kind, items);
            if let Some(p) = &mut self.picker {
                p.title = title;
            }
        }
        (self.message, self.message_path) = (told.message, told.message_path);
    }

    pub(super) fn drop_search(&mut self) {
        let Some(search) = self.background.search.take() else {
            return;
        };
        if search.told.is_none()
            && let Some(searcher) = self.background.searcher.take()
        {
            searcher.dropped.store(true, Ordering::Relaxed);
        }
        if self.message == SEARCHING {
            self.message.clear();
        }
    }

    fn searcher_died(&mut self) {
        self.background.search = None;
        if let Some(searcher) = self.background.searcher.take()
            && let Err(panic) = searcher.thread.join()
        {
            std::panic::resume_unwind(panic);
        }
    }

    pub(super) fn definition_tick(&mut self) -> bool {
        if self.background.search.is_none() {
            return false;
        }
        let place = self.place();
        let Some(search) = &mut self.background.search else {
            return false;
        };
        if search.place != place {
            self.drop_search();
            return true;
        }
        if search.told.is_none() {
            let got = (self.background.searcher.as_ref()).map(|s| s.told.try_recv());
            match got {
                Some(Ok(told)) => search.told = Some(told),
                Some(Err(TryRecvError::Empty)) => {}
                Some(Err(TryRecvError::Disconnected)) | None => {
                    self.searcher_died();
                    return true;
                }
            }
        }
        if self.mode != Mode::Normal || self.picker.is_some() {
            return false;
        }
        if search.told.is_some() {
            self.key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
            self.drop_search();
            return true;
        }
        let said = self.message.is_empty();
        if said {
            self.message = SEARCHING.into();
        }
        said
    }
}
