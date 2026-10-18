#!/usr/bin/env python3
"""Records the reference of the theme bench: the colours LazyVim paints on the same files, per
theme. Once, on a recording machine; never in CI.

    tools/theme-bench/record.py [--theme a,b] [--lang python,go] [--jobs 2] [--cache DIR]
    tools/theme-bench/record.py --check [--theme a,b]
    tools/theme-bench/record.py --fixtures [--lang python]

--fixtures records the golden colours of `cargo test` (#802): every file of tests/colors/LANG/
drawn in the FIXTURE_THEMES, written to tests/colors/LANG/THEME.tsv as colors/ is, with a
character's colour as `rrggbb` followed by `b` when bold, `i` when italic and `/rrggbb` its
background when it is neither Normal's nor CursorLine's (#845).

Every theme of themes.tsv is cloned at its pinned commit into the cache (default
$THEME_BENCH_CACHE or ~/.cache/merl-theme-bench), with nvim-treesitter at NVIM_TS and the
parsers LazyVim installs by default (PARSERS) built with `cc` at the revisions nvim-treesitter
pins, a parser it marks `generate` first through `tree-sitter generate` (the CLI on PATH, 0.26.1 or
later), as nvim-treesitter does. Neovim then starts in tmux on its own server, isolated `XDG_*` under the cache, with only
the colorscheme with its `after/` appended as lazy.nvim's `add_to_rtp` does, nvim-treesitter's queries and parsers, `termguicolors`, `number`, `cursorline`
and `nowrap`, the theme's `before` commands and `:colorscheme`, and `vim.treesitter.start` on
every buffer. Each file is drawn WIDTH columns wide and tall enough for its first LINES lines;
the screen, read as tools/cast.py reads it, gives the foreground of every character that is not
blank, written to colors/THEME/LANG.tsv:

    # palette a=c8d3f5 b=86e1fc ...       the theme's colours in this language, a letter each
    PROJECT/PATH<TAB>LINE:RUNS LINE:RUNS ...

RUNS is the line's non-blank characters in order, a letter per colour and the count when it
repeats (`b4ac10`: four in b, one in a, ten in c). A line is recorded when it is ASCII and at
most MAXLEN columns, so both programs draw all of it. colors/THEME/chrome.tsv holds the
highlight groups the chrome comes from, as Neovim draws them.

The files: FILES per language from the projects of tools/d-bench/projects.tsv and THEME_ONLY
(cloned into $D_BENCH_CACHE or ~/.cache/merl-d-bench, as the other benches do), taken in the order of the hash
of their path, each with at least MINLINES recordable lines among its first LINES, or the
language's own minimum where its projects hold no file that long (Lua).

--check only loads every theme, as tools/port-theme.sh does, and compares its Normal background
and foreground with the shipped themes/NAME.tmTheme: a difference means the row's commit or
commands are not the ones the theme was ported from, or the repo changed since.
"""
import argparse, collections, concurrent.futures, hashlib, json, os, re, shlex, subprocess, sys, tempfile, threading, time

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
NVIM_TS = "1a4f47b03dfe92d28de7d5654c67327ef008c9e6"
PARSERS = ["bash", "c", "css", "diff", "html", "javascript", "jsdoc", "json", "lua", "luadoc", "luap", "markdown",
           "markdown_inline", "printf", "python", "query", "regex", "toml", "tsx", "typescript", "vim",
           "vimdoc", "xml", "yaml", "rust", "go", "cpp", "java", "c_sharp", "kotlin", "swift", "ruby", "php", "php_only", "elixir", "dockerfile", "cmake", "proto", "hcl", "terraform", "ini", "zig", "nix", "scheme", "r", "erlang", "racket", "commonlisp", "sql", "scala", "vue", "svelte", "graphql", "nginx", "gdscript", "starlark", "solidity"]
LANGS = {
    "python": (["paperless-ngx"], ["py"]),
    "rust": (["ripgrep"], ["rs"]),
    "typescript": (["outline"], ["ts"]),
    "tsx": (["outline"], ["tsx"]),
    "javascript": (["eslint"], ["js"]),
    "jsx": (["mastodon"], ["jsx"]),
    "go": (["caddy"], ["go"]),
    "markdown": (["ripgrep", "caddy", "outline", "paperless-ngx"], ["md"]),
    "bash": (["redis", "paperless-ngx", "ripgrep", "caddy", "eslint"], ["sh", "bash"]),
    "json": (["eslint", "paperless-ngx", "redis"], ["json", "jsonc"]),
    "yaml": (["paperless-ngx", "eslint", "caddy", "redis"], ["yaml", "yml"]),
    "toml": (["ripgrep", "paperless-ngx", "eslint", "redis"], ["toml"]),
    "lua": (["redis"], ["lua"], 25),
    "c": (["redis"], ["c"]),
    "cpp": (["leveldb"], ["cc", "h"]),
    "java": (["halo"], ["java"]),
    "c_sharp": (["eShop"], ["cs"]),
    "kotlin": (["nowinandroid"], ["kt", "kts"]),
    "swift": (["Alamofire"], ["swift"]),
    "ruby": (["mastodon"], ["rb"]),
    "php": (["koel"], ["php"]),
    "elixir": (["phoenix"], ["ex", "exs"]),
    "sql": (["immich", "mastodon", "halo"], ["sql"]),
    "hcl": (["immich", "pipeline-library"], ["hcl"], 15),
    "terraform": (["lem", "immich"], ["tf", "tfvars"], 10),
    "ini": (["gitea"], ["ini"], 40),
    "graphql": (["graphql-code-generator", "swapi-graphql"], ["graphql", "graphqls", "gql"], 25),
    "nginx": (["mastodon", "paperless-ngx", "server-configs-nginx"], ["conf", "nginx"], 10),
    "gdscript": (["dialogic"], ["gd"], 40),
    "starlark": (["rules_go"], ["bzl", "bazel", "star", "BUILD", "WORKSPACE"], 40),
    "solidity": (["openzeppelin-contracts"], ["sol"]),
    "proto": (["rules_go", "nowinandroid", "eShop", "gitea"], ["proto"], 25),
    "cmake": (["chibi-scheme", "leveldb", "redis"], ["CMakeLists.txt", "cmake"], 40),
    "dockerfile": (["mastodon", "paperless-ngx", "immich", "gitea"], ["Dockerfile", "dockerfile"]),
    "html": (["bootstrap", "caddy", "eslint", "outline", "redis"], ["html", "htm"]),
    "css": (["gitea", "paperless-ngx", "caddy", "redis"], ["css"]),
    "xml": (["koel", "paperless-ngx", "nowinandroid"], ["xml", "svg", "xsd", "xslt"]),
    "diff": (["rules_go", "openzeppelin-contracts", "outline", "halo"], ["diff", "patch"]),
    "query": (["nvim-treesitter"], ["scm"]),
    "zig": (["zls"], ["zig", "zon"]),
    "nix": (["nix-darwin"], ["nix"]),
    "scheme": (["chibi-scheme"], ["scm", "sld"]),
    "r": (["dplyr"], ["R", "r"]),
    "erlang": (["cowboy"], ["erl", "hrl"]),
    "racket": (["drracket"], ["rkt"]),
    "commonlisp": (["lem"], ["lisp", "asd"]),
    "scala": (["cats"], ["scala", "sbt", "sc", "mill"]),
    "vue": (["gitea"], ["vue"]),
    "svelte": (["immich"], ["svelte"]),
}
NOT_HTML = re.compile(
    r"@(if|for|defer|switch)|\*(ngIf|ngFor|ngSwitch|ngTemplateOutlet)|\bng-(template|content)|\bDTD[ \t]+XHTML[ \t]"
    r"|(?i:\{%[ \t]*(autoescape|block|comment|csrf_token|cycle|debug|extends|filter|firstof|for|if|ifchanged|include"
    r"|load|lorem|now|query_string|regroup|resetcycle|spaceless|templatetag|url|verbatim|widthratio|with)\b)"
    r"|\{#[ \t]+|<extend|<super>")
THEME_ONLY = {
    "nvim-treesitter": ["nvim-treesitter", "query", "https://github.com/nvim-treesitter/nvim-treesitter.git", NVIM_TS, "-"],
    "graphql-code-generator": ["graphql-code-generator", "graphql",
                               "https://github.com/dotansimha/graphql-code-generator.git",
                               "291cfb2a434f24de54a856cc09c6a4dd8ad01526", "-"],
    "swapi-graphql": ["swapi-graphql", "graphql", "https://github.com/graphql/swapi-graphql.git",
                      "8c0bf5868f11fe84942548180751fceeb36bd606", "-"],
    "server-configs-nginx": ["server-configs-nginx", "nginx", "https://github.com/h5bp/server-configs-nginx.git",
                             "d2f2c3e2fac76f429adb738894499b8fb756d2f3", "-"],
    "dialogic": ["dialogic", "gdscript", "https://github.com/dialogic-godot/dialogic.git",
                 "701b67bbe60114e010476d1685f7de1261091c29", "-"],
    "phoenix": ["phoenix", "elixir", "https://github.com/phoenixframework/phoenix.git",
                "e9eeba217bbc89635d8f26404a9471d49617f07f", "-"],
    "zls": ["zls", "zig", "https://github.com/zigtools/zls.git", "eab2be0fd74443a27662da809b771c4ad0d2afcf", "-"],
    "cats": ["cats", "scala", "https://github.com/typelevel/cats.git", "4a2ea736632c64aaf2e4915e6922e3585379382f", "-"],
    "dplyr": ["dplyr", "r", "https://github.com/tidyverse/dplyr.git", "d5e94e7fa8fd4a5f79c1a707d1842216bb4c691f", "-"],
    "cowboy": ["cowboy", "erlang", "https://github.com/ninenines/cowboy.git", "3a001e2fd022c41897c17e88c0e719f53456ff6e", "-"],
}
SKIP_DIRS = ("migrations/", "vendor/", "testdata/", "node_modules/", "CHANGELOG")
FILES, LINES, MINLINES, MAXLEN, WIDTH = 4, 200, 80, 140, 200
LETTERS = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
FIXTURE_THEMES = ["tokyonight-moon", "catppuccin-mocha", "gruvbox-dark", "github-light"]
GROUPS = ["Normal", "CursorLine", "Visual", "Search", "CurSearch", "LineNr", "CursorLineNr", "StatusLine"]
SERVER = f"merl-theme-bench-{os.getpid()}"


def tsv(path):
    return [l.rstrip("\n").split("\t") for l in open(path) if l.strip() and not l.startswith("#")]


def manifest():
    return {r[0]: tuple(r[1:5]) for r in tsv(os.path.join(HERE, "themes.tsv"))}


def git(*args, cwd=None):
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True).stdout.strip()


def ensure(d, url, commit):
    if not os.path.isdir(os.path.join(d, ".git")):
        os.makedirs(d, exist_ok=True)
        subprocess.run(["git", "init", "-q"], cwd=d, check=True)
        subprocess.run(["git", "remote", "add", "origin", url], cwd=d, check=True)
    if git("rev-parse", "HEAD", cwd=d) != commit:
        subprocess.run(["git", "fetch", "-q", "--depth", "1", "origin", commit], cwd=d, check=True)
        subprocess.run(["git", "checkout", "-q", "--detach", "FETCH_HEAD"], cwd=d, check=True)
    return d


def repo(cache, source):
    name, commit = source.split("@")
    return ensure(os.path.join(cache, "themes", name.replace("/", "__")), f"https://github.com/{name}.git", commit)


def projects():
    return {**{r[0]: r for r in tsv(os.path.join(ROOT, "tools/d-bench/projects.tsv"))}, **THEME_ONLY}


def project(name):
    _, _, url, commit, _ = projects()[name]
    cache = os.path.realpath(os.environ.get("D_BENCH_CACHE", os.path.expanduser("~/.cache/merl-d-bench")))
    return ensure(os.path.join(cache, name), url, commit)


def file_lines(path):
    return open(path, encoding="utf-8", errors="replace").read().split("\n")[:LINES]


def recordable(text):
    return text.strip() and text.isascii() and len(text.expandtabs(8)) <= MAXLEN


def pick(lang):
    names, exts, *least = LANGS[lang]
    every = []
    for name in names:
        d = project(name)
        for f in git("ls-files", cwd=d).splitlines():
            if (f.rsplit(".", 1)[-1] in exts or os.path.basename(f) in exts) and not any(s in f for s in SKIP_DIRS):
                every.append((f"{name}/{f}", os.path.join(d, f)))
    every.sort(key=lambda p: hashlib.sha1(p[0].encode()).hexdigest())
    out = []
    for rel, path in every:
        if lang == "html" and any(map(NOT_HTML.search, file_lines(path)[:40])):
            continue
        if lang == "xml" and any(re.search("<!DOCTYPE.*DocBook", l) or "docbook.org/ns/docbook" in l.lower() for l in file_lines(path)[:100]):
            continue
        base = os.path.basename(rel)
        if lang == "nginx" and not (rel.endswith(".nginx") or base.startswith("nginx") or base.endswith("nginx.conf")
                                    or "/nginx/" in rel):
            continue
        if lang == "query" and "/queries/" not in f"/{rel}":
            continue
        if lang == "diff" and re.match(r"From [0-9a-fA-F]{40,} Mon Sep 17 00:00:00 2001$", file_lines(path)[0]):
            continue
        if sum(map(bool, map(recordable, file_lines(path)))) >= (least or [MINLINES])[0]:
            out.append((rel, path))
        if len(out) == FILES:
            break
    return out


def parsers(cache):
    ts = ensure(os.path.join(cache, "nvim-treesitter"), "https://github.com/nvim-treesitter/nvim-treesitter.git",
                NVIM_TS)
    site = os.path.join(cache, "site", "parser")
    os.makedirs(site, exist_ok=True)
    lua = ("local p = dofile(arg[1] .. '/lua/nvim-treesitter/parsers.lua') "
           "for _, l in ipairs(vim.split(arg[2], ',')) do local i = p[l].install_info "
           "io.write(table.concat({l, i.url, i.revision, i.location or '-', i.generate and 'generate' or '-'}, '\\t'), '\\n') end")
    script = os.path.join(cache, "parsers.lua")
    open(script, "w").write(lua)
    r = subprocess.run(["nvim", "--clean", "-l", script, ts, ",".join(PARSERS)], capture_output=True, text=True)
    rows = r.stdout.split("\n")
    for row in filter(None, rows):
        lang, url, rev, loc, generate = row.split("\t")
        stamp = os.path.join(cache, "grammars", f"{lang}.built")
        if os.path.exists(stamp) and open(stamp).read() == rev:
            continue
        g = ensure(os.path.join(cache, "grammars", os.path.basename(url)), url, rev)
        src = os.path.join(g, loc) if loc != "-" else g
        if generate != "-":
            subprocess.run(["tree-sitter", "generate", "--abi", "15", "src/grammar.json"], cwd=src, check=True,
                           env={**os.environ, "TREE_SITTER_JS_RUNTIME": "native"})
        files = [os.path.join(src, "src", f) for f in ("parser.c", "scanner.c") if os.path.exists(os.path.join(src, "src", f))]
        subprocess.run(["cc", "-O2", "-shared", "-fPIC", "-std=c11", "-I", os.path.join(src, "src"), *files,
                        "-o", os.path.join(site, f"{lang}.so")], check=True)
        open(stamp, "w").write(rev)
        print(f"parser {lang} {rev[:12]}", file=sys.stderr)
    return ts


def init_lua(cache, name, ts, check=False):
    source, colorscheme, before, deps = manifest()[name]
    dirs = [repo(cache, source)] + [repo(cache, d) for d in deps.split(",") if d != "-"]
    lines = [f"vim.opt.rtp:prepend({json.dumps(d)})" for d in dirs]
    lines += [f"vim.opt.rtp:append({json.dumps(os.path.join(d, 'after'))})" for d in dirs
              if os.path.isdir(os.path.join(d, "after"))]
    if not check:
        lines += [f"vim.opt.rtp:prepend({json.dumps(ts)})",
                  f"vim.opt.rtp:prepend({json.dumps(os.path.join(ts, 'runtime'))})",
                  f"vim.opt.rtp:prepend({json.dumps(os.path.join(cache, 'site'))})",
                  "vim.g._ts_force_sync_parsing = true", "vim.o.number = true", "vim.o.cursorline = true", "vim.o.wrap = false",
                  "vim.api.nvim_create_autocmd('FileType', { callback = function(a) pcall(vim.treesitter.start, a.buf) end })"]
    lines += ["vim.o.termguicolors = true", "vim.o.swapfile = false", "vim.o.shadafile = 'NONE'"]
    if before != "-":
        lines.append(f"vim.cmd({json.dumps(before)})")
    lines.append(f"vim.cmd.colorscheme({json.dumps(colorscheme)})")
    path = os.path.join(cache, "init", f"{name}{'-check' if check else ''}.lua")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    open(path, "w").write("\n".join(lines) + "\n")
    return path


def xdg(cache, name):
    d = os.path.join(cache, "xdg", name)
    homes = {f"XDG_{k}_HOME": os.path.join(d, k.lower()) for k in ("CONFIG", "DATA", "STATE", "CACHE")}
    return {**homes, "XDG_CONFIG_DIRS": os.path.join(d, "config-dirs"), "XDG_DATA_DIRS": os.path.join(d, "data-dirs")}


def groups(cache, name, init, names=GROUPS):
    out = tempfile.mktemp(suffix=".json")
    lua = ("local o = {} for _, g in ipairs(vim.split(" + json.dumps(",".join(names)) + ", ',')) do "
           "o[g] = vim.api.nvim_get_hl(0, {name = g, link = false}) end "
           f"vim.fn.writefile({{vim.json.encode(o)}}, {json.dumps(out)})")
    subprocess.run(["nvim", "--headless", "-u", init, "-c", f"lua {lua}", "-c", "qa!"],
                   env={**os.environ, **xdg(cache, name)}, check=True, capture_output=True, timeout=60)
    hl = json.load(open(out))
    os.remove(out)
    hexed = lambda v: f"#{v:06x}" if isinstance(v, int) else "-"
    normal = hl["Normal"]

    def drawn(g):
        fg, bg = hl[g].get("fg", normal.get("fg")), hl[g].get("bg", normal.get("bg"))
        return (hexed(bg), hexed(fg)) if hl[g].get("reverse") else (hexed(fg), hexed(bg))
    return {g: drawn(g) for g in names}


def shipped(name):
    text = open(os.path.join(ROOT, "themes", f"{name}.tmTheme")).read()
    key = lambda k: re.search(rf"<key>{k}</key>\s*<string>(#[0-9A-Fa-f]{{6}})", text).group(1).lower()
    return key("foreground"), key("background")


def tmux(*args):
    return subprocess.run(["tmux", "-L", SERVER, "-f", "/dev/null", *args], capture_output=True, text=True)


def server():
    tmux("new-session", "-d", "-s", "idle", "-x", "10", "-y", "2", "sleep 86400")
    tmux("set-option", "-g", "remain-on-exit", "on")


def capture(session, cmd, rows, env, ready, keys=()):
    sys.path.insert(0, os.path.join(ROOT, "tools"))
    import cast
    tmux("kill-session", "-t", session)
    shell = " ".join(f"{k}={shlex.quote(v)}" for k, v in env.items()) + " " + cmd
    r = tmux("new-session", "-d", "-s", session, "-x", str(WIDTH), "-y", str(rows), f"env {shell}")
    if r.returncode:
        sys.exit(r.stderr)
    grids, deadline = [], time.time() + 20
    for key in (None, *keys):
        if key:
            tmux("send-keys", "-t", session, *key)
        seen = []
        while True:
            time.sleep(0.15)
            raw = tmux("capture-pane", "-p", "-e", "-t", session).stdout
            dead = tmux("display-message", "-p", "-t", session, "#{pane_dead} #{pane_dead_status}").stdout.split()
            if dead and dead[0] == "1":
                said = tmux("capture-pane", "-p", "-t", session).stdout.strip()
                tmux("kill-session", "-t", session)
                raise RuntimeError(f"{cmd}: exited with {' '.join(dead[1:])}: {' '.join(said.split())[:300]}")
            grid = cast.parse(raw, WIDTH, rows)
            seen = seen[-2:] + [raw]
            if len(seen) == 3 and len(set(seen)) == 1 and ready(grid):
                break
            if time.time() > deadline:
                tmux("kill-session", "-t", session)
                raise TimeoutError(f"{cmd}: the screen did not settle")
        grids.append(grid)
    tmux("kill-session", "-t", session)
    return grids


def on_screen(grid, lines, plain_bgs=None):
    hits = []
    for cells in grid:
        text = "".join(c[0] for c in cells)
        for m in re.finditer(r"(\d+) ", text):
            n = int(m.group(1))
            if not 1 <= n <= len(lines):
                continue
            want = "".join(lines[n - 1].split())
            got = [(c[0], c[1] if plain_bgs is None else hexed(c[1]) + "b" * c[3] + "i" * c[4]
                    + ("" if hexed(c[2]) in plain_bgs else "/" + hexed(c[2])))
                   for c in cells[m.end():] if not c[0].isspace()]
            if want and "".join(ch for ch, _ in got[:len(want)]) == want:
                hits.append((m.end(), n, [fg for _, fg in got[:len(want)]]))
    if not hits:
        return {}
    gutter = collections.Counter(end for end, _, _ in hits).most_common(1)[0][0]
    return {n: fgs for end, n, fgs in hits if end == gutter}


def hexed(rgb):
    return "%02x%02x%02x" % tuple(rgb)


def encode(fgs, palette):
    out, i = [], 0
    while i < len(fgs):
        j = i
        while j < len(fgs) and fgs[j] == fgs[i]:
            j += 1
        out.append(palette[fgs[i]] + (str(j - i) if j - i > 1 else ""))
        i = j
    return "".join(out)


def decode(runs, colours):
    return [colours[letter] for letter, n in re.findall(r"([A-Za-z])(\d*)", runs) for _ in range(int(n or 1))]


def read_colors(path):
    rows = open(path).read().splitlines()
    pins = dict(p.split("@") for p in next(r for r in rows if r.startswith("# projects")).split()[2:])
    stale = [p for p, c in pins.items() if projects()[p][3] != c]
    if stale:
        raise SystemExit(f"{path}: recorded at other commits of {', '.join(stale)}; record it again")
    colours = dict(p.split("=") for p in next(r for r in rows if r.startswith("# palette")).split()[2:])
    out = {}
    for row in rows:
        if not row or row.startswith("#"):
            continue
        rel, cells = row.split("\t")
        out[rel] = {int(n): decode(runs, colours) for n, runs in (c.split(":") for c in cells.split())}
    return out


def write_colors(path, head, files):
    count = {}
    for _, got in files:
        for fgs in got.values():
            for c in fgs:
                count[c] = count.get(c, 0) + 1
    order = sorted(count, key=lambda c: -count[c])
    if len(order) > len(LETTERS):
        raise ValueError(f"{path}: {len(order)} colours, more than {len(LETTERS)} letters")
    palette = dict(zip(order, LETTERS))
    body = [f"{rel}\t" + " ".join(f"{n}:{encode(got[n], palette)}" for n in sorted(got)) for rel, got in files]
    pal = " ".join(f"{palette[c]}={c}" for c in order)
    open(path, "w").write(head + f"# palette {pal}\n" + "\n".join(body) + "\n")


def record(cache, ts, name, langs):
    init = init_lua(cache, name, ts)
    out = os.path.join(HERE, "colors", name)
    os.makedirs(out, exist_ok=True)
    hl = groups(cache, name, init)
    open(os.path.join(out, "chrome.tsv"), "w").write(
        "# group\tfg\tbg as Neovim draws them: unset from Normal, reverse swapped\n" + "".join(f"{g}\t{fg}\t{bg}\n" for g, (fg, bg) in hl.items()))
    source, colorscheme, before, _ = manifest()[name]
    env = {**xdg(cache, name), "TERM": "tmux-256color"}
    for lang in langs:
        files = []
        for rel, path in pick(lang):
            lines = file_lines(path)
            first = next(n for n, l in enumerate(lines, 1) if recordable(l))
            ready = lambda g: first in on_screen(g, lines)
            grid, = capture(f"r{threading.get_ident()}", f"nvim -u {shlex.quote(init)} {shlex.quote(path)}", LINES + 4, env, ready)
            got = on_screen(grid, lines)
            files.append((rel, {n: [hexed(c) for c in fgs] for n, fgs in got.items() if recordable(lines[n - 1])}))
        version = subprocess.run(["nvim", "--version"], capture_output=True, text=True).stdout.split("\n")[0]
        pins = " ".join(f"{p}@{projects()[p][3]}" for p in LANGS[lang][0])
        head = (f"# {name}: {source} `{before if before != '-' else ''}{' | ' if before != '-' else ''}"
                f"colorscheme {colorscheme}`; {version}, nvim-treesitter {NVIM_TS[:7]}\n# projects {pins}\n")
        write_colors(os.path.join(out, f"{lang}.tsv"), head, files)
    print(f"{name}: {', '.join(langs)}", file=sys.stderr)


def record_fixtures(cache, ts, name, langs):
    init = init_lua(cache, name, ts)
    env = {**xdg(cache, name), "TERM": "tmux-256color"}
    hl = groups(cache, name, init, ["Normal", "CursorLine", "MatchParen"])
    plain = {bg.lstrip("#") for _, bg in hl.values()}
    for lang in langs:
        d = os.path.join(ROOT, "tests", "colors", lang)
        files = []
        for f in sorted(os.path.relpath(os.path.join(w, n), d) for w, _, ns in os.walk(d) for n in ns):
            if f.endswith(".tsv") or f == "NOTES.md":
                continue
            path = os.path.join(d, f)
            lines = file_lines(path)
            bad = [n for n, l in enumerate(lines, 1) if l.strip() and not recordable(l)]
            if bad:
                sys.exit(f"{path}: lines {bad} are not ASCII or longer than {MAXLEN}")
            first = next(n for n, l in enumerate(lines, 1) if recordable(l))
            ready = lambda g: first in on_screen(g, lines)
            grid, = capture(f"f{threading.get_ident()}", f"nvim -u {shlex.quote(init)} {shlex.quote(path)}", LINES + 4,
                            env, ready)
            files.append((f, {n: cells for n, cells in on_screen(grid, lines, plain).items() if recordable(lines[n - 1])}))
        version = subprocess.run(["nvim", "--version"], capture_output=True, text=True).stdout.split("\n")[0]
        source, colorscheme, before, _ = manifest()[name]
        head = (f"# {name}: {source} `{before if before != '-' else ''}{' | ' if before != '-' else ''}"
                f"colorscheme {colorscheme}`; {version}, nvim-treesitter {NVIM_TS[:7]}\n")
        write_colors(os.path.join(d, f"{name}.tsv"), head, files)
    print(f"{name}: fixtures {', '.join(langs)}", file=sys.stderr)


def check(cache, names):
    bad = 0
    for name in names:
        try:
            hl = groups(cache, name, init_lua(cache, name, None, check=True))
            got = hl["Normal"]
        except subprocess.CalledProcessError as e:
            got = ("error", e.stderr.decode()[-80:].strip())
        want = shipped(name)
        ok = got == want
        bad += not ok
        print(f"{name:24} {'ok' if ok else 'DIFF':5} nvim {got[0]} on {got[1]}   shipped {want[0]} on {want[1]}")
    return bad


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--theme", help="themes of themes.tsv, comma-separated (default: every one not skipped)")
    ap.add_argument("--lang", default=",".join(LANGS))
    ap.add_argument("--jobs", type=int, default=2)
    ap.add_argument("--cache", default=os.environ.get("THEME_BENCH_CACHE", os.path.expanduser("~/.cache/merl-theme-bench")))
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--fixtures", action="store_true")
    a = ap.parse_args()
    cache = os.path.realpath(a.cache)
    every = manifest()
    names = a.theme.split(",") if a.theme else [n for n, r in every.items() if r[0] != "skip"]
    unknown = [n for n in names if n not in every or every[n][0] == "skip"]
    if unknown:
        sys.exit(f"not in themes.tsv or skipped: {', '.join(unknown)}")
    if a.check:
        sys.exit(1 if check(cache, names) else 0)
    ts = parsers(cache)
    langs = a.lang.split(",")
    if a.fixtures:
        names = a.theme.split(",") if a.theme else FIXTURE_THEMES
        langs = [l for l in langs if os.path.isdir(os.path.join(ROOT, "tests", "colors", l))]
        server()
        try:
            with concurrent.futures.ThreadPoolExecutor(a.jobs) as pool:
                for f in [pool.submit(record_fixtures, cache, ts, n, langs) for n in names]:
                    f.result()
        finally:
            tmux("kill-server")
        return
    for lang in langs:
        pick(lang)
    server()
    try:
        with concurrent.futures.ThreadPoolExecutor(a.jobs) as pool:
            for f in [pool.submit(record, cache, ts, n, langs) for n in names]:
                f.result()
    finally:
        tmux("kill-server")


if __name__ == "__main__":
    main()
