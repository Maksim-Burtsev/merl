#!/usr/bin/env python3
"""Recording the `d` bench, once, on a recording machine: never in merl, never in CI.

  record.py sample LANG N [--exclude dir,dir]   picks N cursors in LANG's project -> cursors/LANG.tsv
  record.py oracle LANG                         asks a language server -> answers/LANG.tsv

The projects are LANG's rows of projects.tsv, N cursors from each, cloned into the cache by `run`
first ($D_BENCH_CACHE or ~/.cache/merl-d-bench). The servers come from install-servers.sh
($D_BENCH_SERVERS or <cache>/servers). `oracle` resumes from the answers already written.
Java, Kotlin, C#, Ruby, Nix, Groovy, Jenkins (a Groovy shared library, its own row), Emacs Lisp,
Scheme and Common Lisp have no server here: their answers were judged by reading the code, as are
the classes of CSS's Astro templates, which no server answers (`judge` in the note).
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
KW["clojure"] = """def defn defn- defmacro defonce defmulti defmethod defprotocol defrecord deftype declare
ns require import use refer let letfn fn if if-not if-let if-some when when-not when-let when-some cond
condp case do loop recur try catch finally throw quote var new set! and or not nil true false binding
for doseq dotimes doto some-> some->> cond-> cond->> as-> str map mapv filter remove reduce first second
rest next cons conj assoc assoc-in dissoc get get-in update update-in keys vals count seq vec vector list
hash-map hash-set into apply partial comp identity println prn print format nil? some? empty? not=
inc dec merge concat range take drop keyword symbol name meta atom deref swap! reset! instance? fn?
map? vector? string? keyword? symbol? seq? coll? number? int? boolean? ex-info ex-data this""".split()
KW["elisp"] = """defun defmacro defsubst defvar defvar-local defcustom defconst defface defgroup defalias let
let* if when unless cond progn prog1 prog2 setq setq-local setf lambda and or not nil t while dolist
dotimes condition-case ignore-errors unwind-protect save-excursion save-restriction save-match-data
with-current-buffer with-temp-buffer interactive car cdr cadr cddr cons list append concat format message
error user-error funcall apply mapcar mapc memq member assq assoc push pop require provide
declare-function declare eq eql equal null pcase pcase-let pcase-dolist if-let if-let* when-let
when-let* and-let* catch throw quote function cl-loop cl-case cl-defun cl-defmethod cl-defgeneric
insert point goto-char forward-line string-match match-string substring length nth nthcdr
buffer-substring-no-properties current-buffer get-buffer-create""".split()
KW["scheme"] = """define define-syntax define-record-type define-library define-values lambda let let*
letrec letrec* let-values let*-values let-syntax letrec-syntax if cond case when unless begin do set!
quote quasiquote unquote and or not else import export include include-ci syntax-rules
er-macro-transformer car cdr cadr cddr cons list null? pair? list? eq? eqv? equal? apply map for-each
vector string length append reverse error display newline values call-with-values call/cc
call-with-current-continuation dynamic-wind assq assv assoc memq member vector-ref vector-set!
string-ref string-length number? string? symbol? procedure? zero? exact inexact""".split()
KW["racket"] = KW["scheme"] + """racket lang base define/contract define/public define/private define/override
define/augment struct require provide module module+ module* for for/list for/fold for/hash for/and
for/or for*/list match match-define send new class class* object% this super-new init init-field field
inherit public private override augment hash hash-ref hash-set! void parameterize with-handlers
contract-out all-defined-out only-in prefix-in except-in rename-in submod case-lambda
define-syntax-rule syntax-parse syntax-case syntax quasisyntax first second rest empty empty? cons*
printf format eprintf string-append""".split()
KW["commonlisp"] = """defun defmacro defvar defparameter defconstant defclass defmethod defgeneric defstruct
deftype define-condition defpackage in-package let let* flet labels macrolet lambda if when unless cond
case ecase typecase etypecase progn prog1 setf setq incf decf loop do dolist dotimes and or not nil t
declare ignore ignorable type optimize funcall apply list cons car cdr first second rest push pop format
error values multiple-value-bind destructuring-bind handler-case handler-bind unwind-protect with-slots
with-accessors return return-from block eq eql equal gethash make-instance make-hash-table function
quote length nth elt aref string= null listp consp stringp numberp mapcar mapc remove find position
member assoc append nreverse reverse concatenate coerce use export import-from shadow""".split()
KW["bash"] = """if then else elif fi case esac for select while until do done in function time coproc
local declare typeset export readonly unset return exit shift set eval exec source trap wait break continue
echo printf read cd pwd test true false command builtin type hash alias unalias let getopts shopt ulimit umask
kill jobs disown bg fg mapfile readarray caller compgen complete enable help history logout popd pushd dirs
suspend times grep sed awk cat cut tr sort uniq head tail wc rm mv cp mkdir rmdir ln ls find xargs chmod
chown touch date sleep env basename dirname printenv mktemp tee diff curl wget openssl base64 od hexdump
git tput stty uname id whoami ps tar gzip gunzip sha256sum shasum md5sum dd seq expr bc jq python python3
perl ruby node bash sh zsh nc socat ssh scp realpath readlink file stat timeout which install make""".split()
KW["lua"] = """and break do else elseif end false for function goto if in local nil not or repeat return then
true until while self""".split()
KW["js"] = KW["ts"]
for _c in ("vue", "svelte", "astro"):
    KW[_c] = KW["ts"] + "each then key html snippet render debug".split()
KW["jenkins"] = KW["groovy"]
KW["objc"] = C_KW + """self super nil Nil YES NO id instancetype BOOL SEL Class IMP NSInteger NSUInteger
CGFloat interface implementation end property protocol optional required synthesize dynamic
selector encode class import autoreleasepool synchronized nonatomic atomic strong weak copy assign
readonly readwrite nullable nonnull null_resettable __block __weak __strong __kindof
NS_ASSUME_NONNULL_BEGIN NS_ASSUME_NONNULL_END""".split()
KW["dart"] = """abstract as assert async await base break case catch class const continue
covariant default deferred do dynamic else enum export extends extension external factory false
final finally for Function get hide if implements import in interface is late library mixin new
null of on operator part required rethrow return sealed set show static super switch sync this
throw true try type typedef var void when while with yield Object String int double num bool List
Map Set Iterable Future Stream Duration DateTime Uri Exception Error Never Null Type Symbol print
identical override""".split()
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
SPEC["dart"] = dict(exts=(".dart",), lc=("//",), bc=("/*", "*/"), triple=True,
                    decl={"final", "late", "mixin", "get", "set", "typedef", "library", "part"})
LISP_WORD = r"(?![0-9])[A-Za-z0-9_{0}]*[A-Za-z][A-Za-z0-9_{0}]*"
SPEC["clojure"] = dict(exts=(".clj", ".cljs", ".cljc", ".bb"), lc=(";",), bc=None, lisp="\\",
                       word=LISP_WORD.format(r"?!*+<>=\-"))
SPEC["elisp"] = dict(exts=(".el",), lc=(";",), bc=None, lisp="?", word=LISP_WORD.format(r"?!*+<>=/\-"))
SPEC["scheme"] = dict(exts=(".scm", ".sld", ".ss"), lc=(";",), bc=("#|", "|#"), lisp="#\\",
                      word=LISP_WORD.format(r"?!*+<>=/:\-"))
SPEC["racket"] = {**SPEC["scheme"], "exts": (".rkt",)}
SPEC["commonlisp"] = dict(exts=(".lisp", ".lsp", ".cl", ".asd"), lc=(";",), bc=("#|", "|#"), lisp="#\\",
                          word=LISP_WORD.format(r"?!*+<>=/%\-"))
SKIP_DIRS = {".git", "node_modules", "vendor", "third_party", "dist", "build", "target", ".venv",
             "venv", "__pycache__", "migrations", "deps", "public", "static", "locale", "locales",
             "generated", ".build", "Pods", "fixtures", "testdata"}


def nix_mask(text):
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
    if spec.get("nix"):
        text = nix_mask(text)
    out = []
    state = None
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
            if spec.get("lisp") and c == '"':
                state = ("str", c); masked[i] = " "; i += 1; continue
            ch = spec.get("lisp")
            if ch and line.startswith(ch, i) and (ch != "?" or i == 0 or line[i - 1] in " \t()[]{}'`,"):
                j = i + len(ch) + (ch == "?" and line[i + 1:i + 2] == "\\") + 1
                while j < L and line[j].isalpha() and line[j - 1].isalpha():
                    j += 1
                masked[i:min(j, L)] = " " * (min(j, L) - i); i = j; continue
            if c in "\"'" and not ch:
                if spec.get("rust") and c == "'" and not re.match(r"'(\\.|[^\\'])'", line[i:i + 4] if line[i + 1:i + 2] != "\\" else line[i:i + 5]):
                    i += 1; continue
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


def lisp_shape(before, word):
    if before.endswith(("(.", "[.", " .")):
        return "member"
    if before.endswith("(") and not before.endswith(("((", "[(")):
        return "call"
    if before.endswith(("/", ":")):
        return "path"
    return "type" if word[0].isupper() else "name"


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
    return None


CSS_SHAPES = [
    ("name", re.compile(r"\$([A-Za-z_][\w-]*)(?!\s*:)")),
    ("call", re.compile(r"@include\s+([A-Za-z_][\w-]*)")),
    ("call", re.compile(r"(?<![\w$@.#%-])([A-Za-z_][\w-]*)\((?=[^)]*\$)")),
    ("path", re.compile(r"@(?:import|use|forward)\s+[\"']([^\"']+)[\"']")),
]


def sample_css(root, n, exclude=()):
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
    return draw(buckets, {"name": 0.4, "call": 0.25, "path": 0.1, "class": 0.25}, 20261002, n)


SH_VARS = set("BASH_SOURCE FUNCNAME LINENO PWD HOME PATH IFS RANDOM OPTARG OPTIND BASH_VERSION".split())


def sh_tokens(text):
    out, heredoc = [], None
    for n, line in enumerate(text.split("\n"), 1):
        if heredoc:
            if line.strip() == heredoc or line.lstrip("\t") == heredoc:
                heredoc = None
            continue
        m = re.search(r"<<-?\s*['\"]?([A-Za-z_][A-Za-z0-9_]*)['\"]?", line)
        i, L, quote = 0, len(line), None
        code, dq = [True] * L, [False] * L
        while i < L:
            c = line[i]
            if quote == "'":
                code[i] = False
                quote = None if c == "'" else quote
                i += 1; continue
            if c == "\\":
                code[i:i + 2] = [False] * len(code[i:i + 2])
                i += 2; continue
            if quote == '"':
                dq[i] = True
                quote = None if c == '"' else quote
                i += 1; continue
            if c == "'":
                quote = "'"; code[i] = False; i += 1; continue
            if c == '"':
                quote = '"'; dq[i] = True; i += 1; continue
            if c == "#" and (i == 0 or line[i - 1] in " \t;|&(") and not line[max(0, i - 2):i].endswith(("$", "${")):
                code[i:] = [False] * (L - i)
                break
            i += 1
        for t in re.finditer(r"[A-Za-z_][A-Za-z0-9_]*", line):
            s, e = t.span()
            if not code[s] or (s > 0 and (line[s - 1].isalnum() or line[s - 1] in "-./:@")) or line[e:e + 1] == "-":
                continue
            before, after = line[:s], line[e:]
            var = before.endswith(("$", "${", "${#", "${!"))
            if var or not dq[s]:
                out.append((n, s, t.group(), before, after, var))
        if m and not heredoc:
            heredoc = m.group(1)
    return out


def sh_words(text, kw):
    for n, line in enumerate(text.split("\n"), 1):
        r = re.match(r"""\s*(?:source|\.)\s+["']?([^\s;"']*/([A-Za-z_][\w-]*)[\w.-]*)""", line)
        if r and "$(" not in r.group(1):
            yield n, r.start(2), "path", r.group(2)
    for n, s, w, before, after, var in sh_tokens(text):
        if var:
            if w not in SH_VARS:
                yield n, s, "name", w
            continue
        b = before.rstrip()
        cmd = (b == "" or b.endswith((";", "&&", "||", "|", "$(", "`", "{", "(", "!"))
               or re.search(r"(?:^|\s)(?:then|do|else|if|while|until|elif|run|time)$", b))
        if cmd and w not in kw and not re.match(r"\s*(=|\+=|\(\s*\)|\[)", after):
            yield n, s, "call", w


def lua_mask(text):
    out, i, n = list(text), 0, len(text)

    def blank(a, b):
        out[a:b] = [ch if ch == "\n" else " " for ch in text[a:b]]

    while i < n:
        long = re.match(r"(--)?\[(=*)\[", text[i:i + 20])
        if long:
            j = text.find("]" + long.group(2) + "]", i)
            j = n if j < 0 else j + len(long.group(2)) + 2
        elif text.startswith("--", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
        elif text[i] in "\"'":
            j = i + 1
            while j < n and text[j] not in (text[i], "\n"):
                j += 2 if text[j] == "\\" else 1
            j += 1
        else:
            i += 1; continue
        blank(i, j); i = j
    return "".join(out)


def lua_words(text, kw):
    for n, (line, m) in enumerate(zip(text.split("\n"), lua_mask(text).split("\n")), 1):
        for r in re.finditer(r"""\brequire\s*\(?\s*["']([\w./-]+)["']""", line):
            if m[r.start()] != " ":
                last = r.group(1).rsplit(".", 1)[-1]
                yield n, r.end(1) - len(last), "path", last
        for t in re.finditer(r"[A-Za-z_][A-Za-z0-9_]*", m):
            s, e = t.span()
            w, before, after = t.group(), m[:s], m[e:]
            if (w in kw or len(w) < 2 or s > 0 and m[s - 1].isdigit()
                    or re.search(r"\b(?:local|function)\s+(?:[\w.]*[.:])?$", before) and not before.rstrip().endswith(",")
                    or re.search(r"\blocal\s+[\w\s,<>]*,\s*$", before) or re.search(r"\bfor\s+[\w\s,]*$", before)
                    or re.match(r"\s*=[^=]", after) and (before.strip() == "" or before.rstrip().endswith((",", "{")))):
                continue
            if before.endswith((".", ":")) and not before.endswith(".."):
                yield n, s, "member", w
            else:
                yield n, s, "call" if re.match(r"\s*[(\"'{]", after) else "name", w


SPEC["bash"] = dict(exts=(".sh", ".bash", ".bats"), shebang=r"#!.*\b(?:ba)?sh\b", words=sh_words,
                    mix={"call": 0.5, "name": 0.4, "path": 0.1})
SPEC["lua"] = dict(exts=(".lua",), words=lua_words, mix={"member": 0.35, "call": 0.3, "name": 0.25, "path": 0.1},
                   only={"nvim-treesitter": ("lua", "plugin", "scripts"), "kong": ("kong",)})


def script_files(root, spec, only, exclude):
    skip = SKIP_DIRS - {"migrations"}
    for d, dirs, files in os.walk(root):
        dirs[:] = sorted(x for x in dirs if x not in skip and x not in exclude and not x.startswith(".")
                         and (d != root or not only or x in only))
        if d == root and only:
            continue
        for f in sorted(files):
            p = os.path.join(d, f)
            try:
                if not f.endswith(spec["exts"]) and not ("." not in f and spec.get("shebang") and re.match(
                        spec["shebang"], open(p, encoding="utf-8", errors="replace").readline())):
                    continue
                text = open(p, encoding="utf-8", newline="").read()
            except Exception:
                continue
            if len(text) < 400_000:
                yield os.path.relpath(p, root), text


def sample_script(lang, project, root, n, exclude):
    spec, kw, buckets = SPEC[lang], set(KW[lang]), defaultdict(list)
    for rel, text in script_files(root, spec, spec.get("only", {}).get(project, ()), exclude):
        for ln, col, sh, w in spec["words"](text, kw):
            buckets[sh].append((rel, ln, col, sh, w))
    return draw(buckets, spec["mix"], 20261018, n)


def draw(buckets, mix, seed, n):
    rnd = random.Random(seed)
    chosen = []
    for sh, frac in mix.items():
        pool = buckets.get(sh, [])
        chosen += rnd.sample(pool, min(len(pool), round(n * frac)))
    rnd.shuffle(chosen)
    print({k: len(v) for k, v in buckets.items()}, "->", len(chosen), file=sys.stderr)
    return chosen


def sample(lang, project, root, n, exclude=()):
    if lang == "css":
        return sample_css(root, n, exclude)
    spec = SPEC[lang]
    if spec.get("words"):
        return sample_script(lang, project, root, n, exclude)
    kw = set(KW[lang])
    buckets = defaultdict(list)
    for d, dirs, files in os.walk(root):
        dirs[:] = [x for x in dirs if x not in SKIP_DIRS and x not in exclude and not x.startswith(".")]
        for f in files:
            if not f.endswith(spec["exts"]) or f.endswith((".d.ts", ".min.js", "_pb2.py", ".pb.go")):
                continue
            p = os.path.join(d, f)
            try:
                text = open(p, encoding="utf-8", newline="").read()
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
                if spec.get("lisp"):
                    head = re.search(spec["word"] + r"[\s(]*$", before)
                    if (w.startswith(":") or re.search(r"(^|[^A-Za-z0-9_*+!?<>=/%-]):+$", before)
                            or head and re.match(r"(cl-)?def|define", head.group())):
                        continue
                    buckets[lisp_shape(before, w)].append((rel, ln, col, lisp_shape(before, w), w))
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
    mix = {"member": 0.35, "call": 0.25, "type": 0.2, "name": 0.1, "path": 0.1}
    if spec.get("lisp"):
        mix = {"call": 0.5, "name": 0.35, "path": 0.1, "member": 0.025, "type": 0.025}
    return draw(buckets, mix, 20260928, n)


class Lsp:
    def __init__(self, cmd, root, init_options=None, settings=None, env=None):
        self.p = subprocess.Popen(cmd, cwd=root, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=open(os.path.join(CACHE, "lsp-stderr.log"), "ab"),
                                  env={**os.environ, **(env or {})})
        self.id, self.buf, self.progress, self.settings = 0, b"", {}, settings or {}
        self.last_progress = time.time()
        self.opened, self.diagnosed = set(), set()
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
        if meth and "id" in m:
            res = None
            if meth == "workspace/configuration":
                res = [self.section(it.get("section")) for it in m["params"]["items"]]
            elif meth == "workspace/workspaceFolders":
                res = []
            self.send({"jsonrpc": "2.0", "id": m["id"], "result": res})
        elif meth == "textDocument/publishDiagnostics":
            self.diagnosed.add(m["params"]["uri"])
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
           ".bzl": "starlark", ".bazel": "starlark", ".jl": "julia", ".clj": "clojure", ".cljs": "clojure",
           ".cljc": "clojure", ".bb": "clojure", ".rkt": "racket", ".dart": "dart"}


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
    if lang == "bash":
        return Lsp([node, os.path.join(nm, "bash-language-server", "out", "cli.js"), "start"], root,
                   settings={"bashIde": {"backgroundAnalysisMaxFiles": 2000}})
    if lang == "lua":
        return Lsp([os.path.join(LSP, "lua-language-server", "bin", "lua-language-server")], root, settings={"Lua": {
            "runtime": {"version": "LuaJIT", "path": ["?.lua", "?/init.lua", "lua/?.lua", "lua/?/init.lua"]},
            "workspace": {"checkThirdParty": False, "maxPreload": 20000, "preloadFileSize": 2000},
            "diagnostics": {"enable": False}, "telemetry": {"enable": False}}})
    if lang == "clojure":
        edn = open(os.path.join(root, "deps.edn")).read()
        paths = re.findall(r'"([^"]+)"', " ".join(re.findall(r":(?:extra-)?paths\s*\[([^\]]*)\]", edn)))
        cp = ":".join(dict.fromkeys(p for p in paths if os.path.isdir(os.path.join(root, p))))
        return Lsp(["clojure-lsp"], root, init_options={
            "project-specs": [{"project-path": "deps.edn", "classpath-cmd": ["echo", cp]}]})
    if lang == "racket":
        return Lsp(["racket", "-l", "racket-langserver"], root)
    if lang == "julia":
        return Lsp(["julia", "--project=@ls", "-e", "using LanguageServer; runserver(stdin, stdout, pwd())"], root,
                   env={"HOME": os.path.join(LSP, "julia-home")})
    if lang == "dart":
        return Lsp([os.path.join(LSP, "dart-sdk", "bin", "dart"), "--suppress-analytics", "language-server",
                    "--cache=" + os.path.join(LSP, "dart-server-cache"), "--client-id=merl-d-bench"], root,
                   env={"PUB_CACHE": os.path.join(LSP, "dart-pub-cache")})
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


def oracle(lang, project, root, cursors, out, warm=0):
    rows = [r for r in (l.rstrip("\n").split("\t") for l in open(cursors) if not l.startswith("#")) if r[1] == project]
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
    if p.startswith(root + "/"):
        return os.path.relpath(p, root)
    home = os.path.expanduser("~")
    return "~" + p[len(home):] if p.startswith(home + "/") else p


def oracle_run(lang, root, rows, out, warm):
    s = server(lang, root)
    for r in rows[:0 if lang == "bash" else 5]:
        p = os.path.join(root, r[2]); s.open(p, LANG_ID.get(os.path.splitext(p)[1], lang))
    s.settle(quiet=8 if lang in ("rust", "c", "cpp", "objc", "swift", "php", "bash", "lua") else 3)
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
            if lang == "racket":
                end = time.time() + 300
                while "file://" + p not in s.diagnosed and time.time() < end:
                    s.pump(1)
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


def projects(lang):
    rows = [l.rstrip("\n").split("\t") for l in open(os.path.join(HERE, "projects.tsv")) if not l.startswith("#")]
    found = [(r[0], os.path.realpath(os.path.join(CACHE, r[0]))) for r in rows if r[1] == lang]
    if not found:
        raise SystemExit(f"no project for {lang}")
    return found


if __name__ == "__main__":
    a = sys.argv[1:]
    lang = a[1]
    cursors, answers = (os.path.join(HERE, d, f"{lang}.tsv") for d in ("cursors", "answers"))
    if a[0] == "sample":
        ex = a[a.index("--exclude") + 1].split(",") if "--exclude" in a else ()
        rows = [(name, *c) for name, root in projects(lang) for c in sample(lang, name, root, int(a[2]), ex)]
        with open(cursors, "w") as fh:
            fh.write("# id\tproject\tfile\tline\tcol (0-based)\tshape\tword\n")
            for i, (name, rel, ln, col, sh, w) in enumerate(rows):
                fh.write(f"{lang}{i:04d}\t{name}\t{rel}\t{ln}\t{col}\t{sh}\t{w}\n")
    elif a[0] == "oracle":
        for name, root in projects(lang):
            oracle(lang, name, root, cursors, answers)
    else:
        raise SystemExit(__doc__)
