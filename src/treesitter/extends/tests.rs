use super::extension;
use crate::theme;
use crate::treesitter::{Parsed, QUERIES, Styles, compile_text, grammar, lang, source};

#[test]
fn every_theme_s_query_compiles_with_what_it_uses() {
    let mut compiled = std::collections::HashSet::new();
    for theme in theme::names() {
        for (name, _, _) in QUERIES {
            let (Some(grammar), Ok(text)) = (grammar(name), source(name, |r| r.1, Some(theme)))
            else {
                continue;
            };
            if compiled.insert((*name, text.clone())) {
                compile_text(name, grammar, &text).unwrap_or_else(|e| panic!("{theme}: {e}"));
            }
        }
    }
}

#[test]
fn every_theme_s_vendored_query_is_one_it_ships() {
    let on_disk: std::collections::BTreeSet<(String, String)> = std::fs::read_dir("themes")
        .unwrap()
        .flatten()
        .filter_map(|d| {
            Some((
                d.file_name().to_str()?.strip_suffix(".queries")?.to_owned(),
                d.path(),
            ))
        })
        .flat_map(|(theme, dir)| {
            std::fs::read_dir(dir).unwrap().flatten().map(move |f| {
                let lang = f.path().file_stem().unwrap().to_str().unwrap().to_owned();
                (theme.clone(), lang)
            })
        })
        .collect();
    let used: std::collections::BTreeSet<(String, String)> = on_disk
        .iter()
        .filter(|(t, l)| extension(Some(t), l).is_some())
        .cloned()
        .collect();
    assert_eq!(used, on_disk);
    assert!(on_disk.iter().all(|(t, _)| theme::names().any(|n| n == t)));
}

#[test]
fn a_theme_s_query_paints_over_nvim_treesitter_s_and_switching_themes_repaints() {
    let lines = vec!["#[derive(Debug)]".to_owned(), "struct A;".to_owned()];
    let mut parsed = Parsed::new(lang("rust").unwrap(), &lines).unwrap();
    parsed.parse(None);
    let github = theme::load("github-light").unwrap();
    let directive = github.group("@keyword.directive.rust").unwrap().fg;
    let mut derive = |theme: &theme::Theme| {
        let spans = parsed.spans(&lines, 0, 1, &mut Styles::new(theme));
        spans[0].iter().find(|(_, r)| r.contains(&2)).unwrap().0.fg
    };
    assert_eq!(derive(&github), directive);
    let own = theme::Theme {
        builtin: None,
        ..github.clone()
    };
    assert_ne!(derive(&own), directive);
    assert_eq!(derive(&github), directive);
}
