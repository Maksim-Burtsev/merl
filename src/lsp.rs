use serde_json::{Value, json};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

const OPEN_DOCUMENTS: usize = 8;

#[derive(Clone, Copy)]
pub struct Spec {
    pub name: &'static str,
    args: &'static [&'static str],
    says_when_quiescent: bool,
}

const TYPESCRIPT: Spec = Spec {
    name: "typescript-language-server",
    args: &["--stdio"],
    says_when_quiescent: false,
};
const CLANGD: Spec = Spec {
    name: "clangd",
    args: &["--background-index", "-j=8", "--log=error"],
    says_when_quiescent: false,
};
const RUST_ANALYZER: Spec = Spec {
    name: "rust-analyzer",
    args: &[],
    says_when_quiescent: true,
};

pub fn on() -> bool {
    std::env::var_os("MERL_LSP").is_some_and(|v| v == "1")
}

fn millis(var: &str, default: u64) -> Duration {
    Duration::from_millis(
        std::env::var(var)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default),
    )
}

pub fn spec_for(path: &Path) -> Option<Spec> {
    match path.extension()?.to_str()? {
        "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" => Some(TYPESCRIPT),
        "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => Some(CLANGD),
        "rs" => Some(RUST_ANALYZER),
        _ => None,
    }
}

fn language_id(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or_default() {
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "typescriptreact",
        "jsx" => "javascriptreact",
        "js" | "mjs" | "cjs" => "javascript",
        "c" => "c",
        "rs" => "rust",
        _ => "cpp",
    }
}

fn init_options(spec: &Spec) -> Value {
    match spec.name {
        "typescript-language-server" => {
            let mut tsserver = json!({ "useSyntaxServer": "never" });
            if let Ok(path) = std::env::var("MERL_LSP_TSSERVER") {
                tsserver["path"] = json!(path);
            }
            json!({ "tsserver": tsserver, "disableAutomaticTypingAcquisition": true })
        }
        "rust-analyzer" => json!({
            "cachePriming": { "enable": true },
            "checkOnSave": false,
            "procMacro": { "enable": true },
            "cargo": { "buildScripts": { "enable": true } },
        }),
        _ => json!({}),
    }
}

fn on_path(program: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(program))
        .find(|p| p.is_file())
}

pub fn log(line: &str) {
    let Some(path) = std::env::var_os("MERL_LSP_LOG") else {
        return;
    };
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        drop(writeln!(f, "{line}"));
    }
}

fn uri_of(path: &Path) -> String {
    let mut out = String::from("file://");
    for b in path.to_string_lossy().bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~@+:,=()[]!$&'*;".contains(&b) {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn path_of(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = (bytes[i] == b'%')
            .then(|| rest.get(i + 1..i + 3))
            .flatten()
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match hex {
            Some(b) => {
                out.push(b);
                i += 3;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    Some(PathBuf::from(String::from_utf8(out).ok()?))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub path: PathBuf,
    pub line1: usize,
    pub character: usize,
}

pub struct Answer {
    pub places: Vec<Place>,
    pub round_trip: Duration,
}

pub enum Outcome {
    Answer(Answer),
    Loading,
    Slow,
    Gone,
}

fn places(result: &Value) -> Vec<Place> {
    let one = std::slice::from_ref(result);
    let list = result.as_array().map_or(one, Vec::as_slice);
    let mut out: Vec<Place> = Vec::new();
    for r in list {
        let uri = r.get("targetUri").or_else(|| r.get("uri"));
        let range = r.get("targetSelectionRange").or_else(|| r.get("range"));
        let (Some(path), Some(start)) = (
            uri.and_then(Value::as_str).and_then(path_of),
            range.and_then(|r| r.get("start")),
        ) else {
            continue;
        };
        let n = |k: &str| {
            start
                .get(k)
                .and_then(Value::as_u64)
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(0)
        };
        let place = Place {
            path,
            line1: n("line") + 1,
            character: n("character"),
        };
        if !out
            .iter()
            .any(|o| o.path == place.path && o.line1 == place.line1)
        {
            out.push(place);
        }
    }
    out
}

enum Event {
    Message(Value),
    Closed,
    Ask {
        path: PathBuf,
        text: String,
        line: usize,
        character: usize,
        reply: Sender<Answer>,
    },
    Stop,
}

pub struct Client {
    pub spec: Spec,
    events: Sender<Event>,
    ready: Arc<AtomicBool>,
    gone: Arc<AtomicBool>,
    child: Child,
    asked: bool,
    wait: Duration,
    first: Duration,
}

impl Client {
    pub fn start(spec: Spec, root: &Path) -> Option<Self> {
        let program = on_path(spec.name)?;
        let mut command = Command::new(program);
        command
            .args(spec.args)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Some(home) = std::env::var_os("MERL_LSP_HOME") {
            command.env("HOME", home);
        }
        let mut child = command.spawn().ok()?;
        let (stdin, stdout) = (child.stdin.take()?, child.stdout.take()?);
        let (events, inbox) = channel();
        let (ready, gone) = (
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        );
        log(&format!("{} spawn", spec.name));
        let from_server = events.clone();
        std::thread::spawn(move || read_messages(stdout, &from_server));
        let mut worker = Worker {
            spec,
            stdin,
            started: Instant::now(),
            next_id: 1,
            initialized: false,
            queued: Vec::new(),
            pending: HashMap::new(),
            open: VecDeque::new(),
            progress: HashSet::new(),
            answered: false,
            quiescent: false,
            ready: Arc::clone(&ready),
            gone: Arc::clone(&gone),
        };
        let root = root.to_path_buf();
        std::thread::spawn(move || worker.run(&root, &inbox));
        Some(Self {
            spec,
            events,
            ready,
            gone,
            child,
            asked: false,
            wait: millis("MERL_LSP_WAIT_MS", 400),
            first: millis("MERL_LSP_FIRST_MS", 0),
        })
    }

    pub fn ask(&mut self, path: PathBuf, text: String, line: usize, character: usize) -> Outcome {
        if self.gone.load(Ordering::Relaxed) {
            return Outcome::Gone;
        }
        let patient = !self.asked && !self.first.is_zero();
        self.asked = true;
        if patient {
            return self.ask_until_ready(&path, &text, line, character);
        }
        let ready = self.ready.load(Ordering::Relaxed);
        if self.spec.says_when_quiescent && !ready {
            return Outcome::Loading;
        }
        let Some(answer) = self.post(path, text, line, character) else {
            return Outcome::Gone;
        };
        if !ready {
            return Outcome::Loading;
        }
        answer
            .recv_timeout(self.wait)
            .map_or(Outcome::Slow, Outcome::Answer)
    }

    fn post(&self, path: PathBuf, text: String, line: usize, character: usize) -> Option<Receiver<Answer>> {
        let (reply, answer) = channel();
        let ask = Event::Ask {
            path,
            text,
            line,
            character,
            reply,
        };
        self.events.send(ask).ok().map(|()| answer)
    }

    fn ask_until_ready(&self, path: &Path, text: &str, line: usize, character: usize) -> Outcome {
        let deadline = Instant::now() + self.first;
        let mut last = None;
        while Instant::now() < deadline {
            if !self.spec.says_when_quiescent || self.ready.load(Ordering::Relaxed) {
                let Some(answer) = self.post(path.to_path_buf(), text.to_owned(), line, character)
                else {
                    return Outcome::Gone;
                };
                match answer.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                    Ok(a) => last = Some(a),
                    Err(_) => break,
                }
                std::thread::sleep(Duration::from_millis(30));
                if self.ready.load(Ordering::Relaxed) {
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        last.map_or(Outcome::Loading, Outcome::Answer)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        drop(self.events.send(Event::Stop));
        let end = Instant::now() + Duration::from_millis(500);
        while Instant::now() < end && matches!(self.child.try_wait(), Ok(None)) {
            std::thread::sleep(Duration::from_millis(10));
        }
        drop(self.child.kill());
        drop(self.child.wait());
        log(&format!("{} exit", self.spec.name));
    }
}

fn read_messages(stdout: impl Read, events: &Sender<Event>) {
    let mut reader = BufReader::new(stdout);
    loop {
        let mut length = 0;
        loop {
            let mut header = String::new();
            if !matches!(reader.read_line(&mut header), Ok(n) if n > 0) {
                drop(events.send(Event::Closed));
                return;
            }
            let header = header.trim();
            if header.is_empty() {
                break;
            }
            if let Some(n) = header
                .to_ascii_lowercase()
                .strip_prefix("content-length:")
                .and_then(|n| n.trim().parse().ok())
            {
                length = n;
            }
        }
        let mut body = vec![0; length];
        if reader.read_exact(&mut body).is_err() {
            drop(events.send(Event::Closed));
            return;
        }
        if let Ok(message) = serde_json::from_slice(&body)
            && events.send(Event::Message(message)).is_err()
        {
            return;
        }
    }
}

struct Document {
    path: PathBuf,
    hash: u64,
    version: u64,
}

struct Worker {
    spec: Spec,
    stdin: ChildStdin,
    started: Instant,
    next_id: u64,
    initialized: bool,
    queued: Vec<Event>,
    pending: HashMap<u64, (Sender<Answer>, Instant)>,
    open: VecDeque<Document>,
    progress: HashSet<String>,
    answered: bool,
    quiescent: bool,
    ready: Arc<AtomicBool>,
    gone: Arc<AtomicBool>,
}

impl Worker {
    fn note(&self, what: &str) {
        log(&format!(
            "{} +{} {what}",
            self.spec.name,
            self.started.elapsed().as_millis()
        ));
    }

    fn send(&mut self, message: &Value) {
        let body = message.to_string();
        let sent = write!(self.stdin, "Content-Length: {}\r\n\r\n{body}", body.len())
            .and_then(|()| self.stdin.flush());
        if sent.is_err() {
            self.gone.store(true, Ordering::Relaxed);
        }
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    fn run(&mut self, root: &Path, inbox: &Receiver<Event>) {
        let uri = uri_of(root);
        let mut capabilities = json!({
            "textDocument": {
                "definition": { "linkSupport": true },
                "synchronization": { "didSave": true },
            },
            "window": { "workDoneProgress": true },
            "workspace": { "configuration": true, "workspaceFolders": true },
        });
        if self.spec.says_when_quiescent {
            capabilities["experimental"] = json!({ "serverStatusNotification": true });
        }
        let name = root.file_name().unwrap_or_default().to_string_lossy();
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": uri,
                "rootPath": root,
                "capabilities": capabilities,
                "initializationOptions": init_options(&self.spec),
                "workspaceFolders": [{ "uri": uri, "name": name }],
            },
        }));
        while let Ok(event) = inbox.recv() {
            match event {
                Event::Message(m) => self.message(&m),
                Event::Closed => break,
                Event::Stop => {
                    self.send(&json!({ "jsonrpc": "2.0", "id": u64::MAX, "method": "shutdown" }));
                    self.notify("exit", Value::Null);
                    break;
                }
                ask @ Event::Ask { .. } if !self.initialized => self.queued.push(ask),
                Event::Ask {
                    path,
                    text,
                    line,
                    character,
                    reply,
                } => self.ask(&path, &text, line, character, reply),
            }
            let ready = self.initialized
                && self.progress.is_empty()
                && if self.spec.says_when_quiescent {
                    self.quiescent
                } else {
                    self.answered
                };
            if self.ready.swap(ready, Ordering::Relaxed) != ready {
                self.note(if ready { "ready" } else { "unready" });
            }
        }
        self.gone.store(true, Ordering::Relaxed);
        self.ready.store(false, Ordering::Relaxed);
    }

    fn message(&mut self, m: &Value) {
        let method = m.get("method").and_then(Value::as_str);
        let id = m.get("id");
        match (method, id) {
            (Some(method), Some(id)) => {
                let result = match method {
                    "workspace/configuration" => {
                        let items = m["params"]["items"].as_array().map_or(0, Vec::len);
                        json!(vec![Value::Null; items])
                    }
                    "workspace/workspaceFolders" => json!([]),
                    _ => Value::Null,
                };
                self.send(&json!({ "jsonrpc": "2.0", "id": id, "result": result }));
            }
            (Some("$/progress"), None) => {
                let token = m["params"]["token"].to_string();
                match m["params"]["value"]["kind"].as_str() {
                    Some("begin") => {
                        self.progress.insert(token);
                        self.note(&format!("progress-begin open={}", self.progress.len()));
                    }
                    Some("end") => {
                        self.progress.remove(&token);
                        self.note(&format!("progress-end open={}", self.progress.len()));
                    }
                    _ => {}
                }
            }
            (Some("experimental/serverStatus"), None) => {
                self.quiescent = m["params"]["quiescent"].as_bool().unwrap_or(false);
                self.note(&format!("quiescent={}", self.quiescent));
            }
            (None, Some(id)) if id.as_u64() == Some(0) => {
                self.notify("initialized", json!({}));
                self.initialized = true;
                self.note("initialized");
                for ask in std::mem::take(&mut self.queued) {
                    if let Event::Ask {
                        path,
                        text,
                        line,
                        character,
                        reply,
                    } = ask
                    {
                        self.ask(&path, &text, line, character, reply);
                    }
                }
            }
            (None, Some(id)) => {
                let Some((reply, sent)) = id.as_u64().and_then(|id| self.pending.remove(&id))
                else {
                    return;
                };
                let answer = Answer {
                    places: places(&m["result"]),
                    round_trip: sent.elapsed(),
                };
                self.answered = true;
                self.note(&format!(
                    "reply rt={:.1} n={}",
                    answer.round_trip.as_secs_f64() * 1000.0,
                    answer.places.len()
                ));
                drop(reply.send(answer));
            }
            _ => {}
        }
    }

    fn ask(&mut self, path: &Path, text: &str, line: usize, character: usize, reply: Sender<Answer>) {
        let uri = uri_of(path);
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();
        if let Some(at) = self.open.iter().position(|d| d.path == path) {
            let mut doc = self.open.remove(at).expect("the position was just found");
            if doc.hash != hash {
                doc.hash = hash;
                doc.version += 1;
                self.notify(
                    "textDocument/didChange",
                    json!({
                        "textDocument": { "uri": uri, "version": doc.version },
                        "contentChanges": [{ "text": text }],
                    }),
                );
            }
            self.open.push_back(doc);
        } else {
            self.notify(
                "textDocument/didOpen",
                json!({ "textDocument": {
                    "uri": uri, "languageId": language_id(path), "version": 1, "text": text,
                } }),
            );
            self.open.push_back(Document {
                path: path.to_path_buf(),
                hash,
                version: 1,
            });
            if self.open.len() > OPEN_DOCUMENTS
                && let Some(old) = self.open.pop_front()
            {
                self.notify(
                    "textDocument/didClose",
                    json!({ "textDocument": { "uri": uri_of(&old.path) } }),
                );
            }
        }
        self.next_id += 1;
        self.pending.insert(self.next_id, (reply, Instant::now()));
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            },
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_uri_round_trips_and_a_reply_becomes_places() {
        let path = Path::new("/p/node_modules/.pnpm/@a+b@1.0_x/src/a b#é.ts");
        assert_eq!(path_of(&uri_of(path)).as_deref(), Some(path));
        assert_eq!(
            path_of("file:///p/%40scope/a%2Bb.ts"),
            Some(PathBuf::from("/p/@scope/a+b.ts"))
        );
        let link = json!([
            { "targetUri": "file:///p/a.ts", "targetRange": { "start": { "line": 1, "character": 0 } },
              "targetSelectionRange": { "start": { "line": 4, "character": 9 } } },
            { "uri": "file:///p/a.ts", "range": { "start": { "line": 4, "character": 2 } } },
            { "uri": "file:///p/b.ts", "range": { "start": { "line": 0, "character": 0 } } },
        ]);
        let got = places(&link);
        assert_eq!(got.len(), 2);
        assert_eq!((got[0].line1, got[0].character), (5, 9));
        assert_eq!(places(&Value::Null), Vec::new());
        assert_eq!(
            places(&json!({ "uri": "file:///p/c.ts", "range": { "start": { "line": 2, "character": 1 } } }))
                [0]
            .line1,
            3
        );
    }
}
