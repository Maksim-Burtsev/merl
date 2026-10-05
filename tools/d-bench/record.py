#!/usr/bin/env python3
"""Recording the `d` bench, once, on a recording machine: never in merl, never in CI.

  record.py sample LANG N [--exclude dir,dir]   picks N cursors in LANG's project -> cursors/LANG.tsv
  record.py oracle LANG                         asks a language server -> answers/LANG.tsv

The project is LANG's row of projects.tsv, cloned into the cache by `run` first
($D_BENCH_CACHE or ~/.cache/merl-d-bench). The servers come from install-servers.sh
($D_BENCH_SERVERS or <cache>/servers). `oracle` resumes from the answers already written.
Java, Kotlin, C#, Ruby, Nix, Groovy and Jenkins (a Groovy shared library, its own row) have no
server here: their answers were judged by reading the code, as are the classes of CSS's Astro
templates, which no server answers (`judge` in the note).
"""
import json, os, random, re, select, subprocess, sys, time
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
CACHE = os.path.realpath(os.path.expanduser(os.environ.get("D_BENCH_CACHE", "~/.cache/merl-d-bench")))
LSP = os.environ.get("D_BENCH_SERVERS", os.path.join(CACHE, "servers"))

C_KW = """auto break case char const continue default do double else enum extern float for goto if
inline int long register restrict return short signed sizeof static struct switch typedef union
unsigned void volatile while bool true false NULL nullptr class public private protected virtual
override final template typename namespace using new delete this operator friend explicit mutable
constexpr noexcept static_cast dynamic_cast reinterpret_cast const_cast try catch throw decltype
static_assert include define ifdef ifndef endif elif pragma undef defined""".split()
KW = {
    "starlark": """False None True and break continue def elif else for if in lambda load not or pass
return attr ctx native select glob struct depset fail print len str int bool list dict type range
enumerate zip any all min max sorted reversed getattr hasattr Label rule provider aspect
repository_rule module_extension transition""".split(),
    "python": """False None True and as assert async await break class continue def del elif else
except finally for from global if import in is lambda nonlocal not or pass raise return try while
with yield self cls match case print len str int float bool list dict set tuple object type super
isinstance range enumerate zip open any all min max sorted reversed getattr setattr hasattr
Exception""".split(),
    "ts": """break case catch class const continue debugger default delete do else enum export
extends false finally for function if import in instanceof new null return super switch this throw
true try typeof var void while with as implements interface let package private protected public
static yield any boolean number string symbol type from of async await declare readonly keyof
unknown never undefined abstract constructor get set is infer require module namespace console
Promise Array Object String Number Boolean Error Map Set JSON Math Date RegExp""".split(),
    "go": """break case chan const continue default defer else fallthrough for func go goto if
import interface map package range return select struct switch type var nil true false iota int
int8 int16 int32 int64 uint uint8 uint16 uint32 uint64 uintptr float32 float64 string bool byte
rune error any len cap make new append copy delete panic recover close print println complex real
imag min max clear""".split(),
    "rust": """as break const continue crate else enum extern false fn for if impl in let loop
match mod move mut pub ref return self Self static struct super trait true type unsafe use where
while async await dyn Some None Ok Err Box Vec String Option Result i8 i16 i32 i64 i128 isize u8
u16 u32 u64 u128 usize f32 f64 bool char str println format vec write writeln assert assert_eq
panic unreachable todo""".split(),
    "c": C_KW, "cpp": C_KW + "std string vector size_t uint64_t uint32_t int64_t int32_t uint8_t".split(),
    "php": """abstract and array as break callable case catch class clone const continue declare
default do echo else elseif empty enddeclare endfor endforeach endif endswitch endwhile extends
final finally fn for foreach function global goto if implements include include_once instanceof
insteadof interface isset list match namespace new or print private protected public readonly
require require_once return static switch throw trait try unset use var while xor yield true false
null self parent this string int float bool void mixed iterable object never""".split(),
    "swift": """associatedtype class deinit enum extension fileprivate func import init inout
internal let open operator private protocol public rethrows static struct subscript typealias var
break case continue default defer do else fallthrough for guard if in repeat return switch where
while as catch false is nil super self Self throw throws true try await async actor some any
String Int Bool Double Float Array Dictionary Set Optional Void""".split(),
    "java": """abstract assert boolean break byte case catch char class const continue default do
double else enum extends final finally float for goto if implements import instanceof int
interface long native new package private protected public return short static strictfp super
switch synchronized this throw throws transient try void volatile while var record yield true
false null String Object Integer Long Boolean List Map Set Override""".split(),
    "kotlin": """as break class continue do else false for fun if in interface is null object
package return super this throw true try typealias typeof val var when while by catch constructor
delegate dynamic field file finally get import init param property receiver set setparam where
actual abstract annotation companion const crossinline data enum expect external final infix
inline inner internal lateinit noinline open operator out override private protected public
reified sealed suspend tailrec vararg it String Int Long Boolean Unit Any List Map Set""".split(),
    "csharp": """abstract as base bool break byte case catch char checked class const continue
decimal default delegate do double else enum event explicit extern false finally fixed float for
foreach goto if implicit in int interface internal is lock long namespace new null object operator
out override params private protected public readonly ref return sbyte sealed short sizeof
stackalloc static string struct switch this throw true try typeof uint ulong unchecked unsafe
ushort using virtual void volatile while var async await get set init value record nameof where
yield Task List String""".split(),
    "ruby": """alias and begin break case class def defined do else elsif end ensure false for if
in module next nil not or redo rescue retry return self super then true undef unless until when
while yield require require_relative include extend attr_accessor attr_reader attr_writer private
protected public puts raise new lambda proc""".split(),
    "nix": """let in with rec inherit if then else assert or import true false null builtins lib pkgs
config throw abort toString map""".split(),
    "groovy": """abstract as assert boolean break byte case catch char class const continue def default
do double else enum extends false final finally float for goto if implements import in instanceof
int interface long native new null package private protected public return short static strictfp
super switch synchronized this throw throws trait transient true try var void volatile while it
println print String Object Integer Long Boolean List Map Set Closure Override""".split(),
}
KW["solidity"] = """pragma solidity import from as contract abstract interface library is function
modifier event error struct enum type using for mapping returns return emit revert require assert if
else while do break continue new delete public private internal external pure view payable constant
immutable transient virtual override memory storage calldata indexed anonymous unchecked assembly
try catch constructor fallback receive true false this super msg tx block abi address bool string
bytes int uint wei gwei ether seconds minutes hours days weeks keccak256 sha256 ecrecover gasleft
blockhash addmod mulmod selfdestruct length push pop""".split() + [
    f"{t}{n}" for t in ("int", "uint") for n in range(8, 257, 8)] + [f"bytes{n}" for n in range(1, 33)]
KW["julia"] = """abstract baremodule begin break catch const continue do else elseif end export false
finally for function global if import in isa let local macro module mutable primitive quote return
struct true try type using where while nothing missing Any Int Int64 Float64 String Bool Symbol
Nothing Vector Matrix Array Tuple Dict println print length size eltype""".split()
KW["js"] = KW["ts"]
for _c in ("vue", "svelte", "astro"):
    KW[_c] = KW["ts"] + "each then key html snippet render debug".split()
KW["jenkins"] = KW["groovy"]
KW["objc"] = C_KW + """self super nil Nil YES NO id instancetype BOOL SEL Class IMP NSInteger NSUInteger
CGFloat interface implementation end property protocol optional required synthesize dynamic
selector encode class import autoreleasepool synchronized nonatomic atomic strong weak copy assign
readonly readwrite nullable nonnull null_resettable __block __weak __strong __kindof
NS_ASSUME_NONNULL_BEGIN NS_ASSUME_NONNULL_END""".split()
DECL = set("""def class func fn function struct interface type let const var val fun enum trait impl
mod namespace union typedef record object protocol extension typealias module macro_rules define
package import use from""".split())
SOL_DECL = {"contract", "library", "modifier", "event", "error", "is"}
SPEC = {
    "python": dict(exts=(".py",), lc=("#",), bc=None, triple=True),
    "ts": dict(exts=(".ts", ".tsx"), lc=("//",), bc=("/*", "*/"), tmpl="`"),
    "js": dict(exts=(".js", ".mjs", ".cjs", ".jsx"), lc=("//",), bc=("/*", "*/"), tmpl="`"),
    "go": dict(exts=(".go",), lc=("//",), bc=("/*", "*/"), tmpl="`"),
    "rust": dict(exts=(".rs",), lc=("//",), bc=("/*", "*/"), rust=True),
    "c": dict(exts=(".c", ".h"), lc=("//",), bc=("/*", "*/")),
    "cpp": dict(exts=(".cc", ".cpp", ".h", ".hpp"), lc=("//",), bc=("/*", "*/")),
    "objc": dict(exts=(".m", ".h"), lc=("//",), bc=("/*", "*/")),
    "php": dict(exts=(".php",), lc=("//", "#"), bc=("/*", "*/")),
    "swift": dict(exts=(".swift",), lc=("//",), bc=("/*", "*/"), triple=True),
    "java": dict(exts=(".java",), lc=("//",), bc=("/*", "*/"), triple=True),
    "kotlin": dict(exts=(".kt", ".kts"), lc=("//",), bc=("/*", "*/"), triple=True),
    "csharp": dict(exts=(".cs",), lc=("//",), bc=("/*", "*/"), triple=True),
    "ruby": dict(exts=(".rb",), lc=("#",), bc=None),
    "nix": dict(exts=(".nix",), lc=(), nix=True, word=r"[A-Za-z_][A-Za-z0-9_'-]*"),
    "groovy": dict(exts=(".groovy", ".gvy", ".gradle", "Jenkinsfile"), lc=("//",), bc=("/*", "*/"),
                   triple=True, dollar_slashy=True),
    "solidity": dict(exts=(".sol",), lc=("//",), bc=("/*", "*/")),
    "starlark": dict(exts=(".bzl", ".bazel", "BUILD"), lc=("#",), bc=None, triple=True),
    "julia": dict(exts=(".jl",), lc=("#",), bc=("#=", "=#"), triple=True, adjoint=True,
                  word=r"[A-Za-z_][A-Za-z0-9_]*(?:!(?!=))?", decl={"macro", "using"}),
}
NIX_PATH = re.compile(r"(?:^|(?<=[\s(\[{=;]))(?:\.{1,2}|[\w.+-]*)(?:/[\w.+-]+)+/?")
NIX_BINDS = re.compile(r"\s*(?:\.\s*[\w'-]*\s*)*=(?!=)")
for _c in ("vue", "svelte", "astro"):
    SPEC[_c] = dict(SPEC["ts"], exts=(f".{_c}",), component=True)
SPEC["jenkins"] = SPEC["groovy"]
SKIP_DIRS = {".git", "node_modules", "vendor", "third_party", "dist", "build", "target", ".venv",
             "venv", "__pycache__", "migrations", "deps", "public", "static", "locale", "locales",
             "generated", ".build", "Pods", "fixtures", "testdata"}


def nix_mask(text):
    """Blanks Nix's comments and strings, nested in `${}` included, keeping the code inside `${}`."""
    out, stack, i, n = list(text), [0], 0, len(text)

    def blank(a, b):
        out[a:b] = [ch if ch == "\n" else " " for ch in text[a:b]]

    while i < n:
        top = stack[-1]
        if top in ('"', "''"):
            if text.startswith("${", i):
                blank(i, i + 2); stack.append(0); i += 2; continue
            if top == '"' and text[i] == "\\":
                blank(i, i + 2); i += 2; continue
            if top == '"' and text[i] == '"' or top == "''" and text.startswith("''", i) and text[i + 2:i + 3] not in ("'", "$", "\\"):
                blank(i, i + len(top)); stack.pop(); i += len(top); continue
            k = 3 if top == "''" and text.startswith("''", i) else 1
            blank(i, i + k); i += k; continue
        if text[i] == "#" or text.startswith("/*", i):
            j = text.find("\n", i) if text[i] == "#" else text.find("*/", i + 2) + 2
            j = n if j < 2 else j
            blank(i, j); i = j; continue
        if text.startswith("''", i) or text[i] == '"':
            k = 2 if text[i] == "'" else 1
            stack.append(text[i:i + k]); blank(i, i + k); i += k; continue
        if text[i] == "}" and top == 0 and len(stack) > 1:
            blank(i, i + 1); stack.pop(); i += 1; continue
        stack[-1] += {"{": 1, "}": -1}.get(text[i], 0)
        i += 1
    return "".join(out)


def code_tokens(text, spec):
    """(line1, col, word, before, after) for identifiers outside comments and strings."""
    if spec.get("nix"):
        text = nix_mask(text)
    out = []
    state = None  # None | 'block' | ('str', closer)
    for n, line in enumerate(text.split("\n"), 1):
        i, L = 0, len(line)
        masked = list(line)
        while i < L:
            c = line[i]
            if state == "block":
                j = line.find(spec["bc"][1], i)
                if j < 0:
                    masked[i:] = " " * (L - i); i = L; break
                masked[i:j + 2] = " " * (j + 2 - i); i = j + 2; state = None; continue
            if isinstance(state, tuple):
                closer = state[1]
                j = i
                while j < L:
                    if line[j] == "\\": j += 2; continue
                    if line.startswith(closer, j): break
                    j += 1
                if j >= L:
                    masked[i:] = " " * (L - i); i = L; break
                masked[i:j + len(closer)] = " " * (j + len(closer) - i); i = j + len(closer); state = None; continue
            if spec.get("bc") and line.startswith(spec["bc"][0], i):
                state = "block"; masked[i:i + 2] = "  "; i += 2; continue
            if any(line.startswith(lc, i) for lc in spec["lc"]):
                masked[i:] = " " * (L - i); break
            if spec.get("triple") and (line.startswith('"""', i) or line.startswith("'''", i)):
                state = ("str", line[i:i + 3]); masked[i:i + 3] = "   "; i += 3; continue
            if spec.get("nix") and c == "'":
                i += 1; continue
            if spec.get("dollar_slashy") and line.startswith("$/", i):
                state = ("str", "/$"); masked[i:i + 2] = "  "; i += 2; continue
            if spec.get("tmpl") and c == spec["tmpl"]:
                state = ("str", c); masked[i] = " "; i += 1; continue
            if spec.get("adjoint") and c == "'" and i and (line[i - 1].isalnum() or line[i - 1] in "_)]}'."):
                i += 1; continue
            if c in "\"'":
                if spec.get("rust") and c == "'" and not re.match(r"'(\\.|[^\\'])'", line[i:i + 4] if line[i + 1:i + 2] != "\\" else line[i:i + 5]):
                    i += 1; continue  # a lifetime
                j = i + 1
                while j < L and line[j] != c:
                    j += 2 if line[j] == "\\" else 1
                masked[i:min(j + 1, L)] = " " * (min(j + 1, L) - i); i = j + 1; continue
            i += 1
        m = "".join(masked)
        if not m.isascii():
            continue
        if spec.get("nix"):
            for t in NIX_PATH.finditer(m):
                out.append((n, t.start(), t.group(), m[:t.start()], m[t.end():]))
            m = NIX_PATH.sub(lambda t: " " * len(t.group()), m)
        for t in re.finditer(spec.get("word", r"[A-Za-z_][A-Za-z0-9_]*"), m):
            s, e = t.span()
            if s > 0 and (m[s - 1].isdigit() or m[s - 1] in "$@#"):
                continue
            out.append((n, s, t.group(), m[:s], m[e:]))
    return out


def blank(text, a, b):
    return text[:a] + re.sub(r"[^\n]", " ", text[a:b]) + text[b:]


def braces(text):
    out, depth, start, q, i = [], 0, 0, None, 0
    while i < len(text):
        c = text[i]
        if q:
            if c == "\\":
                i += 1
            elif c == q or c == "\n" and q != "`":
                q = None
        elif depth and c in "\"'`":
            q = c
        elif c == "{":
            depth += 1
            if depth == 1:
                start = i + 1
        elif c == "}" and depth:
            depth -= 1
            if not depth:
                out.append((start, i))
        i += 1
    return out


def component_code(text, lang):
    kept = re.sub(r"[^\n]", " ", text)
    keep = lambda a, b: kept[:a] + text[a:b] + kept[b:]
    tpl = text
    for m in re.finditer(r"<script\b[^>]*>(.*?)</script>", text, re.S):
        kept = keep(*m.span(1))
        tpl = blank(tpl, *m.span())
    if lang == "astro":
        m = re.match(r"---\n(.*?)\n---", text, re.S)
        if m:
            kept = keep(*m.span(1))
            tpl = blank(tpl, *m.span())
    for m in re.finditer(r"<style\b.*?</style>|<!--.*?-->", tpl, re.S):
        tpl = blank(tpl, *m.span())
    if lang == "vue":
        spans = [m.span(1) for m in re.finditer(r"\{\{(.*?)\}\}", tpl, re.S)]
        spans += [m.span(1) for m in re.finditer(
            r"\s(?:v-[\w-]+(?::[\w.\[\]-]+)?|[:@#][\w.\[\]-]*)=\"([^\"]*)\"", tpl)]
    else:
        spans = braces(tpl)
    spans += [m.span(1) for m in re.finditer(r"</?([A-Z][\w.]*)", tpl)]
    for a, b in spans:
        kept = keep(a, b)
    return kept


def shape(before, after, lang):
    b = before.rstrip()
    if b.endswith("?.") or (b.endswith(".") and not b.endswith("..")) or b.endswith("->"):
        return "member"
    if b.endswith("::"):
        return "path"
    if after.lstrip().startswith("("):
        return "call"
    return None  # decided by the word


CSS_SHAPES = [
    ("name", re.compile(r"\$([A-Za-z_][\w-]*)(?!\s*:)")),
    ("call", re.compile(r"@include\s+([A-Za-z_][\w-]*)")),
    ("call", re.compile(r"(?<![\w$@.#%-])([A-Za-z_][\w-]*)\((?=[^)]*\$)")),
    ("path", re.compile(r"@(?:import|use|forward)\s+[\"']([^\"']+)[\"']")),
]


def sample_css(project, root, n, out, exclude=()):
    """Stylesheet names in scss/ (variables, mixins, functions, imports, for the oracle) and the
    classes of site/'s Astro templates (judged by reading the stylesheets)."""
    buckets = defaultdict(list)
    declared = set(re.findall(r"@function\s+([\w-]+)", subprocess.run(
        ["grep", "-rh", "@function", os.path.join(root, "scss")], capture_output=True, text=True).stdout))
    for d, dirs, files in os.walk(root):
        dirs[:] = [x for x in dirs if x not in SKIP_DIRS and x not in exclude and not x.startswith(".")]
        rel_dir = os.path.relpath(d, root)
        for f in files:
            rel = os.path.normpath(os.path.join(rel_dir, f))
            text = open(os.path.join(d, f), encoding="utf-8", errors="replace").read()
            if rel.startswith("scss/") and f.endswith(".scss"):
                for ln, line in enumerate(text.split("\n"), 1):
                    code = line.split("//", 1)[0]
                    if not code.isascii():
                        continue
                    for sh, rx in CSS_SHAPES:
                        for m in rx.finditer(code):
                            if rx is CSS_SHAPES[2][1] and m.group(1) not in declared:
                                continue
                            if sh == "name" and re.match(r"\s*\$[\w-]+\s*:", code) and m.start() == code.index("$"):
                                continue
                            buckets[sh].append((rel, ln, m.start(1), sh, m.group(1)))
            elif rel.startswith("site/") and f.endswith(".astro"):
                for ln, line in enumerate(text.split("\n"), 1):
                    if not line.isascii():
                        continue
                    for m in re.finditer(r'\bclass="([^"{}]*)"', line):
                        for w in re.finditer(r"[^\s]+", m.group(1)):
                            buckets["class"].append((rel, ln, m.start(1) + w.start(), "class", w.group()))
    rnd = random.Random(20261002)
    mix = {"name": 0.4, "call": 0.25, "path": 0.1, "class": 0.25}
    chosen = []
    for sh, frac in mix.items():
        pool = buckets.get(sh, [])
        chosen += rnd.sample(pool, min(len(pool), round(n * frac)))
    rnd.shuffle(chosen)
    with open(out, "w") as fh:
        fh.write("# id\tproject\tfile\tline\tcol (0-based)\tshape\tword\n")
        for i, (rel, ln, col, sh, w) in enumerate(chosen):
            fh.write(f"css{i:04d}\t{project}\t{rel}\t{ln}\t{col}\t{sh}\t{w}\n")
    print({k: len(v) for k, v in buckets.items()}, "->", len(chosen), file=sys.stderr)


def sample(lang, project, root, n, out, exclude=()):
    if lang == "css":
        return sample_css(project, root, n, out, exclude)
    spec = SPEC[lang]
    kw = set(KW[lang])
    buckets = defaultdict(list)
    for d, dirs, files in os.walk(root):
        dirs[:] = [x for x in dirs if x not in SKIP_DIRS and x not in exclude and not x.startswith(".")]
        for f in files:
            if not f.endswith(spec["exts"]) or f.endswith((".d.ts", ".min.js", "_pb2.py", ".pb.go")):
                continue
            p = os.path.join(d, f)
            try:
                text = open(p, encoding="utf-8").read()
            except Exception:
                continue
            if len(text) > 400_000:
                continue
            rel = os.path.relpath(p, root)
            if spec.get("component"):
                text = component_code(text, lang)
            for (ln, col, w, before, after) in code_tokens(text, spec):
                if w in kw or len(w) < 2:
                    continue
                if "/" in w:
                    buckets["path"].append((rel, ln, col, "path", w))
                    continue
                if lang == "nix" and NIX_BINDS.match(after):
                    continue
                if spec.get("component") and (re.search(r"</?$", before) and w[0].islower()
                                              or re.match(r"=[\"'{]|-[a-z]", after)):
                    continue
                prev = re.findall(r"[A-Za-z_]+", before)
                decl = (DECL | SOL_DECL if lang == "solidity" else DECL) | spec.get("decl", set())
                if prev and prev[-1] in decl and not before.rstrip().endswith((".", "->", "::", "(", ",", "=", ":")):
                    continue
                sh = shape(before, after, lang) or ("type" if w[0].isupper() else "name")
                buckets[sh].append((rel, ln, col, sh, w))
    rnd = random.Random(20260928)
    mix = {"member": 0.35, "call": 0.25, "type": 0.2, "name": 0.1, "path": 0.1}
    chosen = []
    for sh, frac in mix.items():
        pool = buckets.get(sh, [])
        chosen += rnd.sample(pool, min(len(pool), round(n * frac)))
    rnd.shuffle(chosen)
    with open(out, "w") as fh:
        fh.write("# id\tproject\tfile\tline\tcol (0-based)\tshape\tword\n")
        for i, (rel, ln, col, sh, w) in enumerate(chosen):
            fh.write(f"{lang}{i:04d}\t{project}\t{rel}\t{ln}\t{col}\t{sh}\t{w}\n")
    print({k: len(v) for k, v in buckets.items()}, "->", len(chosen), file=sys.stderr)


class Lsp:
    def __init__(self, cmd, root, init_options=None, settings=None, env=None):
        self.p = subprocess.Popen(cmd, cwd=root, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=open(os.path.join(CACHE, "lsp-stderr.log"), "ab"),
                                  env={**os.environ, **(env or {})})
        self.id, self.buf, self.progress, self.settings = 0, b"", {}, settings or {}
        self.last_progress = time.time()
        self.opened = set()
        root_uri = "file://" + root
        caps = {"textDocument": {"definition": {"linkSupport": True},
                                 "declaration": {"linkSupport": True},
                                 "synchronization": {"didSave": True}},
                "window": {"workDoneProgress": True},
                "workspace": {"configuration": True, "workspaceFolders": True}}
        r = self.request("initialize", {"processId": os.getpid(), "rootUri": root_uri, "rootPath": root,
                                        "capabilities": caps, "initializationOptions": init_options or {},
                                        "workspaceFolders": [{"uri": root_uri, "name": os.path.basename(root)}]},
                         timeout=300)
        self.notify("initialized", {})
        if settings:
            self.notify("workspace/didChangeConfiguration", {"settings": settings})

    def send(self, msg):
        body = json.dumps(msg).encode()
        self.p.stdin.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
        self.p.stdin.flush()

    def notify(self, method, params):
        self.send({"jsonrpc": "2.0", "method": method, "params": params})

    def read(self, timeout):
        end = time.time() + timeout
        while True:
            if b"\r\n\r\n" in self.buf:
                head, rest = self.buf.split(b"\r\n\r\n", 1)
                ln = int(re.search(rb"Content-Length: *(\d+)", head, re.I).group(1))
                if len(rest) >= ln:
                    self.buf = rest[ln:]
                    return json.loads(rest[:ln])
            left = end - time.time()
            if left <= 0:
                return None
            r, _, _ = select.select([self.p.stdout], [], [], left)
            if not r:
                return None
            chunk = os.read(self.p.stdout.fileno(), 1 << 20)
            if not chunk:
                raise RuntimeError("server exited")
            self.buf += chunk

    def handle(self, m):
        meth = m.get("method")
        if meth and "id" in m:  # a request from the server
            res = None
            if meth == "workspace/configuration":
                res = [self.section(it.get("section")) for it in m["params"]["items"]]
            elif meth == "workspace/workspaceFolders":
                res = []
            self.send({"jsonrpc": "2.0", "id": m["id"], "result": res})
        elif meth == "$/progress":
            tok, v = str(m["params"]["token"]), m["params"]["value"]
            self.last_progress = time.time()
            if v.get("kind") == "begin":
                self.progress[tok] = v.get("title", "")
            elif v.get("kind") == "end":
                self.progress.pop(tok, None)

    def section(self, name):
        cur = self.settings
        for part in (name or "").split("."):
            if not part:
                continue
            cur = cur.get(part, {}) if isinstance(cur, dict) else {}
        return cur if cur != {} else None

    def request(self, method, params, timeout=60):
        self.id += 1
        rid = self.id
        self.send({"jsonrpc": "2.0", "id": rid, "method": method, "params": params})
        end = time.time() + timeout
        while time.time() < end:
            m = self.read(end - time.time())
            if m is None:
                break
            if m.get("id") == rid and "method" not in m:
                return m.get("result")
            self.handle(m)
        return "TIMEOUT"

    def pump(self, secs):
        end = time.time() + secs
        while time.time() < end:
            m = self.read(min(0.5, end - time.time()))
            if m:
                self.handle(m)

    def settle(self, quiet=5, cap=900):
        """Waits until no progress has run for `quiet` seconds (or `cap` passed)."""
        start = time.time()
        while time.time() - start < cap:
            self.pump(1)
            if not self.progress and time.time() - self.last_progress > quiet:
                return
        print("settle: cap reached with", self.progress, file=sys.stderr)

    def open(self, path, lang_id):
        if path in self.opened:
            return
        self.opened.add(path)
        text = open(path, encoding="utf-8", errors="replace").read()
        self.notify("textDocument/didOpen", {"textDocument": {"uri": "file://" + path, "languageId": lang_id,
                                                               "version": 1, "text": text}})


LANG_ID = {".py": "python", ".ts": "typescript", ".tsx": "typescriptreact", ".js": "javascript",
           ".jsx": "javascriptreact", ".mjs": "javascript", ".cjs": "javascript", ".go": "go",
           ".rs": "rust", ".c": "c", ".h": "cpp", ".cc": "cpp", ".cpp": "cpp", ".hpp": "cpp",
           ".php": "php", ".swift": "swift", ".m": "objective-c", ".css": "css", ".scss": "scss",
           ".less": "less", ".vue": "vue", ".svelte": "svelte", ".astro": "astro", ".sol": "solidity",
           ".bzl": "starlark", ".bazel": "starlark", ".jl": "julia"}


def server(lang, root):
    node = "node"
    nm = os.path.join(LSP, "node_modules")
    if lang == "python":
        venv = os.path.join(root, ".venv", "bin", "python")
        py = {"python": {"pythonPath": venv} if os.path.exists(venv) else {},
              "python.analysis": {}}
        py["python"]["analysis"] = {"autoSearchPaths": True, "diagnosticMode": "openFilesOnly",
                                    "extraPaths": [os.path.join(root, "src")]}
        return Lsp([node, os.path.join(nm, "pyright", "langserver.index.js"), "--stdio"], root, settings=py)
    if lang in ("ts", "js"):
        return Lsp([node, os.path.join(nm, "typescript-language-server", "lib", "cli.mjs"), "--stdio"], root,
                   init_options={"tsserver": {"path": os.path.join(nm, "typescript", "lib", "tsserver.js")}})
    tsdk = {"typescript": {"tsdk": os.path.join(nm, "typescript", "lib")}}
    if lang == "vue":
        return Lsp([node, os.path.join(nm, "@vue", "language-server", "bin", "vue-language-server.js"), "--stdio"],
                   root, init_options={**tsdk, "vue": {"hybridMode": False}})
    if lang == "svelte":
        return Lsp([node, os.path.join(nm, "svelte-language-server", "bin", "server.js"), "--stdio"], root)
    if lang == "astro":
        return Lsp([node, os.path.join(nm, "@astrojs", "language-server", "bin", "nodeServer.js"), "--stdio"], root,
                   init_options=tsdk)
    if lang == "go":
        return Lsp([os.path.join(LSP, "bin", "gopls")], root, env={"GOFLAGS": "-mod=mod"})
    if lang == "rust":
        return Lsp([os.path.join(LSP, "bin-ra")], root,
                   init_options={"cachePriming": {"enable": True}, "checkOnSave": False,
                                 "procMacro": {"enable": True}, "cargo": {"buildScripts": {"enable": True}}})
    if lang in ("c", "cpp", "objc"):
        return Lsp(["clangd", "--background-index", "-j=8", "--log=error"], root)
    if lang == "php":
        st = os.path.join(CACHE, "intelephense-storage")
        return Lsp([node, os.path.join(nm, "intelephense", "lib", "intelephense.js"), "--stdio"], root,
                   init_options={"storagePath": st, "globalStoragePath": st})
    if lang == "swift":
        return Lsp(["xcrun", "sourcekit-lsp"], root)
    if lang == "css":
        return Lsp([os.path.join(nm, ".bin", "vscode-css-language-server"), "--stdio"], root)
    if lang == "solidity":
        server = os.path.join(nm, "@nomicfoundation", "solidity-language-server", "out", "index.js")
        return Lsp([node, server, "--stdio"], root)
    if lang == "starlark":
        return Lsp([os.path.join(LSP, "bin", "starpls"), "server"], root)
    if lang == "julia":
        return Lsp(["julia", "--project=@ls", "-e", "using LanguageServer; runserver(stdin, stdout, pwd())"], root,
                   env={"HOME": os.path.join(LSP, "julia-home")})
    raise SystemExit(f"no server for {lang}")


def locs(res):
    if res in (None, "TIMEOUT"):
        return []
    if isinstance(res, dict):
        res = [res]
    out = []
    for r in res:
        uri = r.get("targetUri") or r.get("uri")
        rng = r.get("targetSelectionRange") or r.get("range")
        if uri and uri.startswith("file://") and rng:
            from urllib.parse import unquote
            out.append((os.path.realpath(unquote(uri[7:])), rng["start"]["line"] + 1))
    return out


def oracle(lang, root, cursors, out, warm=0):
    rows = [l.rstrip("\n").split("\t") for l in open(cursors) if not l.startswith("#")]
    if not os.path.exists(out):
        with open(out, "w") as fh:
            fh.write("# id\ttargets: path:line joined by ; (project-relative; ~/ or / outside the project;"
                     " empty: the oracle found none)\tskip: why the row is not scored\tnote\n")
    for attempt in range(6):
        done = {l.split("\t", 1)[0] for l in open(out) if not l.startswith("#")}
        todo = [r for r in rows if r[0] not in done]
        if not todo:
            return
        try:
            oracle_run(lang, root, todo, out, warm)
            return
        except RuntimeError as e:
            print("restart after", e, file=sys.stderr)


def short(p, root):
    """Project-relative inside the project, ~/ under the home directory."""
    if p.startswith(root + "/"):
        return os.path.relpath(p, root)
    home = os.path.expanduser("~")
    return "~" + p[len(home):] if p.startswith(home + "/") else p


def oracle_run(lang, root, rows, out, warm):
    s = server(lang, root)
    # Open a few files first so servers that index lazily start their work.
    for r in rows[:5]:
        p = os.path.join(root, r[2]); s.open(p, LANG_ID.get(os.path.splitext(p)[1], lang))
    s.settle(quiet=8 if lang in ("rust", "c", "cpp", "objc", "swift", "php") else 3)
    if warm:
        s.pump(warm)
    with open(out, "a") as fh:
        for k, r in enumerate(rows):
            cid, _, rel, line, col = r[:5]
            p = os.path.join(root, rel)
            if lang == "css" and r[5] == "class":
                fh.write(f"{cid}\t\t\tjudge\n")
                continue
            s.open(p, LANG_ID.get(os.path.splitext(p)[1], lang))
            pos = {"textDocument": {"uri": "file://" + p}, "position": {"line": int(line) - 1, "character": int(col)}}
            got = locs(s.request("textDocument/definition", pos, timeout=60))
            if lang in ("c", "cpp", "objc"):
                got += locs(s.request("textDocument/declaration", pos, timeout=60))
            tag = ""
            if not got and lang in ("rust", "swift", "php"):
                s.pump(2)
                got = locs(s.request("textDocument/definition", pos, timeout=60)); tag = "retry"
            fh.write(f"{cid}\t{';'.join(f'{short(a, root)}:{b}' for a, b in dict.fromkeys(got))}\t\t{tag}\n")
            fh.flush()
            if k % 50 == 0:
                print(k, len(rows), file=sys.stderr)
    s.p.kill()


def project(lang):
    for l in open(os.path.join(HERE, "projects.tsv")):
        r = l.rstrip("\n").split("\t")
        if r[1] == lang:
            return r[0], os.path.join(CACHE, r[0])
    raise SystemExit(f"no project for {lang}")


if __name__ == "__main__":
    a = sys.argv[1:]
    name, root = project(a[1])
    root = os.path.realpath(root)
    if a[0] == "sample":
        ex = a[a.index("--exclude") + 1].split(",") if "--exclude" in a else ()
        sample(a[1], name, root, int(a[2]), os.path.join(HERE, "cursors", f"{a[1]}.tsv"), ex)
    elif a[0] == "oracle":
        oracle(a[1], root, os.path.join(HERE, "cursors", f"{a[1]}.tsv"),
               os.path.join(HERE, "answers", f"{a[1]}.tsv"))
    else:
        raise SystemExit(__doc__)
