use std::path::Path;
use std::sync::OnceLock;

use super::pattern::lua_pattern;
use super::{Lang, lang};

pub fn for_syntax(
    syntect_name: &str,
    path: Option<&Path>,
    lines: &[String],
) -> Option<&'static Lang> {
    let astro = path.is_some_and(|p| p.extension().is_some_and(|e| e == "astro"));
    if path.is_some_and(|p| p.extension().is_none()) {
        let by_hashbang = lines.first().and_then(|l| hashbang(l)).and_then(|name| {
            ["ruby", "php"]
                .into_iter()
                .find(|interpreter| name.contains(interpreter))
        });
        if let Some(name) = by_hashbang {
            return lang(name);
        }
    }
    match syntect_name {
        "Bourne Again Shell (bash)" if !path.is_some_and(zsh) => lang("bash"),
        "C" => lang("c"),
        "C#" => lang("c_sharp"),
        "C++" if cpp(path, lines) => lang("cpp"),
        "CMake" if path.is_none_or(cmake) => lang("cmake"),
        "CSS" => lang("css"),
        "Diff" if diff(path, lines) => lang("diff"),
        "Dockerfile" | "Dockerfile (with bash)" => lang("dockerfile"),
        "Lisp" if path.is_some_and(query) => lang("query"),
        "Lisp" if path.is_some_and(scheme) => lang("scheme"),
        "Lisp" if path.is_some_and(lisp) => lang("commonlisp"),
        "HTML" | "XML" => html_or_xml(path?, lines),
        "Elixir" if !euphoria(path, lines) => lang("elixir"),
        "Erlang" if path.is_none_or(|p| erlang(p, lines)) => lang("erlang"),
        "GDScript (Godot Engine)" => lang("gdscript"),
        "Go" => lang("go"),
        "GraphQL" => lang("graphql"),
        "INI" if path.is_some_and(ini) => lang("ini"),
        "Java" if path.is_none_or(|p| p.extension().is_none_or(|e| e != "bsh")) => lang("java"),
        "JSON" => lang("json"),
        "Kotlin" => lang("kotlin"),
        "nginx" if path.is_some_and(nginx) => lang("nginx"),
        "TOML" => lang("toml"),
        "YAML" => lang("yaml"),
        "JavaScript (Babel)" => lang("javascript"),
        "Python" if path.is_some_and(starlark) => lang("starlark"),
        "Python" => lang("python"),
        "PHP" if path.is_none_or(php) => lang("php"),
        "R" if r(path, lines) => lang("r"),
        "Racket"
            if path.is_none_or(|p| {
                p.extension()
                    .is_some_and(|e| e == "rkt" || e == "rktd" || e == "rktl")
            }) =>
        {
            lang("racket")
        }
        "Protocol Buffer" if path.is_none_or(|p| p.extension().is_some_and(|e| e == "proto")) => {
            lang("proto")
        }
        "Ruby" if path.is_none_or(ruby) => lang("ruby"),
        "Rust" => lang("rust"),
        "Solidity" => lang("solidity"),
        "SQL" if path.is_none_or(sql) => lang("sql"),
        "Scala" if !supercollider(path, lines) => lang("scala"),
        "Swift" => lang("swift"),
        "Terraform" => terraform(path, lines),
        "Markdown" => lang("markdown"),
        "TypeScript" => lang("typescript"),
        "TypeScriptReact" if !astro => lang("tsx"),
        "Lua" => lang("lua"),
        "Zig" => lang("zig"),
        "Nix" => lang("nix"),
        "Vue Component" => lang("vue"),
        "Svelte" => lang("svelte"),
        _ => None,
    }
}

fn terraform(path: Option<&Path>, lines: &[String]) -> Option<&'static Lang> {
    match path?.extension()?.to_str()? {
        "hcl" => lang("hcl"),
        "tfvars" => lang("terraform"),
        "tf" if lines.iter().any(|l| {
            let l = l.trim_start();
            !l.is_empty() && !l.starts_with([';', '/'])
        }) =>
        {
            lang("terraform")
        }
        _ => None,
    }
}

fn nginx(path: &Path) -> bool {
    let full = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let full = full.to_string_lossy();
    let tail = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let conf = path.extension().is_some_and(|e| e == "conf");
    path.extension().is_some_and(|e| e == "nginx")
        || tail.ends_with("nginx.conf")
        || (tail.starts_with("nginx") && conf)
        || full.contains("/etc/nginx/")
        || full.contains("/usr/local/nginx/conf/")
        || (full.contains("/nginx/") && conf)
}

fn starlark(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    ["WORKSPACE", "WORKSPACE.bzlmod", "BUCK", "BUILD"].contains(&name)
        || path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
            [
                "bzl", "bxl", "bazel", "BUILD", "ipd", "sky", "star", "starlark",
            ]
            .contains(&e)
        })
}

fn ini(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "ini" || e == "INI")
        || path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
            [
                ".coveragerc",
                ".npmrc",
                ".pylintrc",
                ".gitlint",
                "setup.cfg",
                "pudb.cfg",
                ".oelint.cfg",
                ".wakatime.cfg",
            ]
            .contains(&n)
        })
}

pub fn hashbang(first: &str) -> Option<&str> {
    let mut words = first.strip_prefix("#!")?.split_whitespace();
    let program = words.next()?.rsplit(['/', '\\']).next()?;
    if program != "env" {
        return Some(program);
    }
    words
        .find(|w| !w.starts_with('-') && !w.contains('='))?
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
}

fn html_variant(lines: &[String]) -> bool {
    static VARIANT: OnceLock<regex::Regex> = OnceLock::new();
    let re = VARIANT.get_or_init(|| {
        regex::Regex::new(concat!(
            r"@(if|for|defer|switch)|\*(ngIf|ngFor|ngSwitch|ngTemplateOutlet)|\bng-(template|content)",
            r"|\bDTD[ \t]+XHTML[ \t]",
            r"|(?i:\{%[ \t]*(autoescape|block|comment|csrf_token|cycle|debug|extends|filter|firstof|for|if",
            r"|ifchanged|include|load|lorem|now|query_string|regroup|resetcycle|spaceless|templatetag|url",
            r"|verbatim|widthratio|with)\b)|\{#[ \t]+|<extend|<super>",
        ))
        .expect("a valid regex")
    });
    lines.iter().take(40).any(|l| re.is_match(l))
}

fn supercollider(path: Option<&Path>, lines: &[String]) -> bool {
    static PATTERNS: OnceLock<Vec<regex::bytes::Regex>> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| {
        [
            "var%s<",
            "classvar%s<",
            "%^this.*",
            "|%w+|",
            "%+%s%w*%s{",
            "%*ar%s",
        ]
        .into_iter()
        .map(|p| lua_pattern(p).expect("Neovim's detect.sc patterns are Lua patterns"))
        .collect()
    });
    path.is_some_and(|p| p.extension().is_some_and(|e| e == "sc"))
        && lines
            .iter()
            .take(25)
            .any(|l| patterns.iter().any(|re| re.is_match(l.as_bytes())))
}

fn euphoria(path: Option<&Path>, lines: &[String]) -> bool {
    path.is_some_and(|p| p.extension().is_some_and(|e| e == "ex"))
        && lines.iter().take(100).any(|l| {
            let lower = l.to_lowercase();
            lower.starts_with("--")
                || ["ifdef", "include"].iter().any(|w| {
                    lower.strip_prefix(w).is_some_and(|rest| {
                        !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_')
                    })
                })
        })
}

fn r(path: Option<&Path>, lines: &[String]) -> bool {
    static REBOL: OnceLock<regex::Regex> = OnceLock::new();
    let Some(path) = path else { return true };
    if !path.extension().is_some_and(|e| e == "r" || e == "R") {
        return path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| [".Rprofile", "Rprofile", "Rprofile.site", ".Rhistory"].contains(&n));
    }
    let rebol = REBOL.get_or_init(|| regex::Regex::new(r"(?i)\brebol\b").expect("a valid regex"));
    let head = || lines.iter().take(50);
    !head().any(|l| rebol.is_match(l))
        && head()
            .map(|l| l.trim_start())
            .find(|l| l.starts_with('#') || l.starts_with("/*"))
            .is_none_or(|l| l.starts_with('#'))
}

fn erlang(path: &Path, lines: &[String]) -> bool {
    path.extension().is_some_and(|e| e == "erl" || e == "hrl")
        || lines
            .first()
            .and_then(|l| hashbang(l))
            .is_some_and(|p| p == "escript")
}

fn cmake(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name == "CMakeLists.txt" || path.extension().is_some_and(|e| e == "cmake")
}

fn sql(path: &Path) -> bool {
    const EXTENSIONS: &[&str] = &["sql", "zsql", "tyc", "pkb", "tyb", "pks"];
    path.file_name().is_some_and(|n| n == ".sqlite_history")
        || path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| EXTENSIONS.contains(&e))
}

fn php(path: &Path) -> bool {
    const EXTENSIONS: &[&str] = &["php", "phtml", "phpt", "php3", "php4", "php5", "php7"];
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    !name.ends_with(".blade.php")
        && path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| EXTENSIONS.contains(&e))
}

fn ruby(path: &Path) -> bool {
    const EXTENSIONS: &[&str] = &["rb", "rbi", "rjs", "ru", "gemspec", "rake"];
    const NAMES: &[&str] = &[
        "Brewfile",
        "Gemfile",
        "irbrc",
        "rakefile",
        "Rakefile",
        "rantfile",
        "Rantfile",
        "Vagrantfile",
    ];
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e))
        || NAMES.contains(&name)
}

fn cpp(path: Option<&Path>, lines: &[String]) -> bool {
    match path.and_then(|p| p.extension()).and_then(|e| e.to_str()) {
        Some("cp") => false,
        Some("h") => !lines.iter().take(200).any(|l| {
            let lower = l.to_lowercase();
            ["@interface", "@end", "@class"]
                .iter()
                .any(|m| lower.starts_with(m))
        }),
        _ => true,
    }
}

const XML_EXTENSIONS: &[&str] = &[
    "svg", "xsd", "xsl", "xslt", "psc1", "tpm", "xliff", "atom", "xul", "cdxml", "mpd", "rss",
    "fsproj", "ui", "vbproj", "xlf", "wsdl", "csproj", "wpl", "xmi", "xpr", "xpfm", "spfm", "bxml",
    "mmi", "xcu", "xlb", "xlc", "xba", "slnx",
];
const XML_NAMES: &[&str] = &[
    "fglrxrc",
    "fonts.conf",
    "Directory.Packages.props",
    "Directory.Build.props",
    "Directory.Build.targets",
];
const XML_PATH_ENDS: &[&str] = &[
    "/etc/blkid.tab",
    "/etc/blkid.tab.old",
    ".csproj.user",
    ".fsproj.user",
    ".vbproj.user",
];

fn html_or_xml(path: &Path, lines: &[String]) -> Option<&'static Lang> {
    static SMIL: OnceLock<regex::Regex> = OnceLock::new();
    let smil = SMIL.get_or_init(|| regex::Regex::new(r"<\?\s*xml.*\?>").expect("a valid regex"));
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let full = path.to_string_lossy();
    let first = lines.first().map_or("", String::as_str);
    let xml = match ext {
        "cpt" | "dtml" | "htm" | "html" | "pt" | "shtml" | "stm" => {
            return if html_variant(lines) {
                None
            } else {
                lang("html")
            };
        }
        "xml" => !docbook_or_xbl(lines),
        "lpr" | "grc" | "ts" => first.contains("<?xml"),
        "smil" => smil.is_match(first),
        _ => {
            XML_EXTENSIONS.contains(&ext)
                || XML_NAMES.contains(&name)
                || XML_PATH_ENDS.iter().any(|end| full.ends_with(end))
                || (full.contains("/etc/xdg/menus/") && ext == "menu")
        }
    };
    if xml { lang("xml") } else { None }
}

fn docbook_or_xbl(lines: &[String]) -> bool {
    lines.iter().take(100).any(|l| {
        let lower = l.to_lowercase();
        l.split_once("<!DOCTYPE")
            .is_some_and(|(_, doctype)| doctype.contains("DocBook"))
            || lower.contains(r#" xmlns="http://docbook.org/ns/docbook""#)
            || lower.contains(r#"xmlns:xbl="http://www.mozilla.org/xbl""#)
    })
}

fn scheme(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e == "scm" || e == "ss" || e == "sld")
}

fn lisp(path: &Path) -> bool {
    const EXTENSIONS: &[&str] = &["lisp", "el", "lsp", "asd", "L"];
    const NAMES: &[&str] = &[".emacs", "sbclrc", ".sbclrc", ".sawfishrc"];
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e))
        || path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| NAMES.contains(&n))
}

fn query(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "scm")
        && path
            .parent()
            .is_some_and(|d| d.components().any(|c| c.as_os_str() == "queries"))
}

fn diff(path: Option<&Path>, lines: &[String]) -> bool {
    static SEND_EMAIL: OnceLock<regex::Regex> = OnceLock::new();
    let send_email = SEND_EMAIL.get_or_init(|| {
        regex::Regex::new(r"^From [0-9a-fA-F]{40,} Mon Sep 17 00:00:00 2001$")
            .expect("a valid regex")
    });
    let Some(path) = path else { return true };
    let patch = path.extension().is_some_and(|e| e == "patch");
    if patch && lines.first().is_some_and(|l| send_email.is_match(l)) {
        return false;
    }
    let dep3 = path.to_string_lossy().contains("/debian/patches/")
        && path.file_name().is_some_and(|n| n != "series")
        && lines
            .iter()
            .take(100)
            .take_while(|l| !l.starts_with("---"))
            .any(|l| DEP3_HEADERS.iter().any(|h| l.starts_with(h)));
    !dep3
}

const DEP3_HEADERS: &[&str] = &[
    "Description:",
    "Subject:",
    "Origin:",
    "Bug:",
    "Forwarded:",
    "Author:",
    "From:",
    "Reviewed-by:",
    "Acked-by:",
    "Last-Updated:",
    "Applied-Upstream:",
];

fn zsh(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let bare = name.strip_prefix('.').unwrap_or(name);
    let ext = name.rsplit_once('.').map_or("", |(_, e)| e);
    matches!(ext, "zsh" | "zunit" | "zsh-theme")
        || bare.starts_with("zsh")
        || bare.starts_with("zlog")
        || name.starts_with(".zcompdump")
        || matches!(name, ".zprofile" | ".zfbfmarks")
        || path == Path::new("/etc/zprofile")
}

#[cfg(test)]
mod tests;
