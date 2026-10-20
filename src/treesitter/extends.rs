#[cfg(not(target_family = "wasm"))]
use std::sync::OnceLock;

#[cfg(target_family = "wasm")]
use crate::web::WasmOnce as OnceLock;

use super::{Lang, QUERIES, compile_text, source};

#[rustfmt::skip]
const EXTENSIONS: &[(&str, &[(&str, &str)])] = &[
    ("alabaster-light", &[
        ("bash", include_str!("../../themes/alabaster-light.queries/bash.scm")),
        ("c_sharp", include_str!("../../themes/alabaster-light.queries/c_sharp.scm")),
        ("c", include_str!("../../themes/alabaster-light.queries/c.scm")),
        ("commonlisp", include_str!("../../themes/alabaster-light.queries/commonlisp.scm")),
        ("cpp", include_str!("../../themes/alabaster-light.queries/cpp.scm")),
        ("go", include_str!("../../themes/alabaster-light.queries/go.scm")),
        ("java", include_str!("../../themes/alabaster-light.queries/java.scm")),
        ("javascript", include_str!("../../themes/alabaster-light.queries/javascript.scm")),
        ("json", include_str!("../../themes/alabaster-light.queries/json.scm")),
        ("kotlin", include_str!("../../themes/alabaster-light.queries/kotlin.scm")),
        ("lua", include_str!("../../themes/alabaster-light.queries/lua.scm")),
        ("python", include_str!("../../themes/alabaster-light.queries/python.scm")),
        ("ruby", include_str!("../../themes/alabaster-light.queries/ruby.scm")),
        ("rust", include_str!("../../themes/alabaster-light.queries/rust.scm")),
        ("scala", include_str!("../../themes/alabaster-light.queries/scala.scm")),
        ("typescript", include_str!("../../themes/alabaster-light.queries/typescript.scm")),
        ("zig", include_str!("../../themes/alabaster-light.queries/zig.scm")),
    ]),
    ("alabaster", &[
        ("bash", include_str!("../../themes/alabaster.queries/bash.scm")),
        ("c_sharp", include_str!("../../themes/alabaster.queries/c_sharp.scm")),
        ("c", include_str!("../../themes/alabaster.queries/c.scm")),
        ("commonlisp", include_str!("../../themes/alabaster.queries/commonlisp.scm")),
        ("cpp", include_str!("../../themes/alabaster.queries/cpp.scm")),
        ("go", include_str!("../../themes/alabaster.queries/go.scm")),
        ("java", include_str!("../../themes/alabaster.queries/java.scm")),
        ("javascript", include_str!("../../themes/alabaster.queries/javascript.scm")),
        ("json", include_str!("../../themes/alabaster.queries/json.scm")),
        ("kotlin", include_str!("../../themes/alabaster.queries/kotlin.scm")),
        ("lua", include_str!("../../themes/alabaster.queries/lua.scm")),
        ("python", include_str!("../../themes/alabaster.queries/python.scm")),
        ("ruby", include_str!("../../themes/alabaster.queries/ruby.scm")),
        ("rust", include_str!("../../themes/alabaster.queries/rust.scm")),
        ("scala", include_str!("../../themes/alabaster.queries/scala.scm")),
        ("typescript", include_str!("../../themes/alabaster.queries/typescript.scm")),
        ("zig", include_str!("../../themes/alabaster.queries/zig.scm")),
    ]),
    ("catppuccin-frappe", &[
        ("css", include_str!("../../themes/catppuccin-frappe.queries/css.scm")),
        ("javascript", include_str!("../../themes/catppuccin-frappe.queries/javascript.scm")),
        ("typescript", include_str!("../../themes/catppuccin-frappe.queries/typescript.scm")),
    ]),
    ("catppuccin-latte", &[
        ("css", include_str!("../../themes/catppuccin-latte.queries/css.scm")),
        ("javascript", include_str!("../../themes/catppuccin-latte.queries/javascript.scm")),
        ("typescript", include_str!("../../themes/catppuccin-latte.queries/typescript.scm")),
    ]),
    ("catppuccin-macchiato", &[
        ("css", include_str!("../../themes/catppuccin-macchiato.queries/css.scm")),
        ("javascript", include_str!("../../themes/catppuccin-macchiato.queries/javascript.scm")),
        ("typescript", include_str!("../../themes/catppuccin-macchiato.queries/typescript.scm")),
    ]),
    ("catppuccin-mocha", &[
        ("css", include_str!("../../themes/catppuccin-mocha.queries/css.scm")),
        ("javascript", include_str!("../../themes/catppuccin-mocha.queries/javascript.scm")),
        ("typescript", include_str!("../../themes/catppuccin-mocha.queries/typescript.scm")),
    ]),
    ("github-dark", &[
        ("bash", include_str!("../../themes/github-dark.queries/bash.scm")),
        ("jsx", include_str!("../../themes/github-dark.queries/jsx.scm")),
        ("php", include_str!("../../themes/github-dark.queries/php.scm")),
        ("rust", include_str!("../../themes/github-dark.queries/rust.scm")),
    ]),
    ("github-light", &[
        ("bash", include_str!("../../themes/github-light.queries/bash.scm")),
        ("jsx", include_str!("../../themes/github-light.queries/jsx.scm")),
        ("php", include_str!("../../themes/github-light.queries/php.scm")),
        ("rust", include_str!("../../themes/github-light.queries/rust.scm")),
    ]),
    ("jellybeans-light", &[
        ("bash", include_str!("../../themes/jellybeans-light.queries/bash.scm")),
    ]),
    ("jellybeans", &[
        ("bash", include_str!("../../themes/jellybeans.queries/bash.scm")),
    ]),
    ("moonfly", &[
        ("css", include_str!("../../themes/moonfly.queries/css.scm")),
        ("html", include_str!("../../themes/moonfly.queries/html.scm")),
    ]),
    ("nightfly", &[
        ("css", include_str!("../../themes/nightfly.queries/css.scm")),
        ("html", include_str!("../../themes/nightfly.queries/html.scm")),
    ]),
    ("solarized-dark", &[
        ("css", include_str!("../../themes/solarized-dark.queries/css.scm")),
        ("javascript", include_str!("../../themes/solarized-dark.queries/javascript.scm")),
        ("lua", include_str!("../../themes/solarized-dark.queries/lua.scm")),
        ("tsx", include_str!("../../themes/solarized-dark.queries/tsx.scm")),
    ]),
    ("solarized-light", &[
        ("css", include_str!("../../themes/solarized-light.queries/css.scm")),
        ("javascript", include_str!("../../themes/solarized-light.queries/javascript.scm")),
        ("lua", include_str!("../../themes/solarized-light.queries/lua.scm")),
        ("tsx", include_str!("../../themes/solarized-light.queries/tsx.scm")),
    ]),
    ("vesper", &[
        ("elixir", include_str!("../../themes/vesper.queries/elixir.scm")),
        ("javascript", include_str!("../../themes/vesper.queries/javascript.scm")),
        ("typescript", include_str!("../../themes/vesper.queries/typescript.scm")),
    ]),
    ("vscode-dark", &[
        ("javascript", include_str!("../../themes/vscode-dark.queries/javascript.scm")),
        ("jsx", include_str!("../../themes/vscode-dark.queries/jsx.scm")),
        ("typescript", include_str!("../../themes/vscode-dark.queries/typescript.scm")),
    ]),
    ("vscode-light", &[
        ("javascript", include_str!("../../themes/vscode-light.queries/javascript.scm")),
        ("jsx", include_str!("../../themes/vscode-light.queries/jsx.scm")),
        ("typescript", include_str!("../../themes/vscode-light.queries/typescript.scm")),
    ]),
];

pub fn extension(theme: Option<&str>, lang: &str) -> Option<&'static str> {
    let (_, files) = EXTENSIONS.iter().find(|(t, _)| Some(*t) == theme)?;
    files
        .iter()
        .find(|(l, _)| *l == lang)
        .map(|(_, text)| *text)
}

pub fn themed(lang: &'static Lang, theme: Option<&str>) -> &'static Lang {
    static THEMED: [[OnceLock<Option<Lang>>; QUERIES.len()]; EXTENSIONS.len()] =
        [const { [const { OnceLock::new() }; QUERIES.len()] }; EXTENSIONS.len()];
    let Some(t) = EXTENSIONS.iter().position(|(t, _)| Some(*t) == theme) else {
        return lang;
    };
    let Some(l) = QUERIES.iter().position(|(n, _, _)| *n == lang.name) else {
        return lang;
    };
    THEMED[t][l]
        .get_or_init(|| {
            let pick = |r: &(&str, &'static str, &'static str)| r.1;
            let text = source(lang.name, pick, theme).ok()?;
            if source(lang.name, pick, None).ok()? == text {
                return None;
            }
            compile_text(lang.name, lang.language.clone(), &text).ok()
        })
        .as_ref()
        .unwrap_or(lang)
}

#[cfg(test)]
mod tests;
