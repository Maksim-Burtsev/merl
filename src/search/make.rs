//! Makefile rules beside the line patterns: what sets a variable only in addition or for some
//! targets, and where a recipe's shell command starts.

pub fn make_fallback_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    let target = r"(?:[^\s:=#$(){}]+|\$[({][^)}]*[)}])+";
    vec![
        format!(r"^\s*(export\s+|override\s+)?{w}\s*\+="),
        format!(
            r"^{target}(?:\s+{target})*\s*::?\s*((export|override|private)\s+)*{w}\s*(\+|[:?!]{{0,3}})="
        ),
    ]
}
/// When 1-based `line` of a Makefile is a recipe line, a shell command that declares nothing
/// make knows, the line its command starts on: a line that starts with a tab after a rule, until
/// a line that is neither a recipe line, a blank, a comment nor a conditional ends the rule, as
/// GNU make reads it, and the lines a `\` continues it over, which one shell runs. A tab-indented
/// assignment inside an `ifeq` before any rule is make's own, and so is the continuation of one.
pub fn make_recipe_command(text: &str, line: usize) -> Option<usize> {
    let mut in_rule = false;
    // The previous line ended in `\`: where the command it belongs to starts, if it is a recipe.
    let mut continues = None;
    for (i, l) in text.lines().take(line).enumerate() {
        let recipe = match continues {
            Some(recipe) => recipe,
            None if l.starts_with('\t') => in_rule.then_some(i + 1),
            None => {
                let t = l.trim_start();
                let conditional = ["ifeq", "ifneq", "ifdef", "ifndef", "else", "endif"]
                    .iter()
                    .any(|d| {
                        t.strip_prefix(d)
                            .is_some_and(|r| r.is_empty() || r.starts_with([' ', '\t', '(']))
                    });
                if !t.is_empty() && !t.starts_with('#') && !conditional {
                    in_rule = starts_rule(t);
                }
                None
            }
        };
        if i + 1 == line {
            return recipe;
        }
        continues = l.ends_with('\\').then_some(recipe);
    }
    None
}
/// Whether a Makefile line outside a recipe is a rule, `targets: prerequisites`, and not an
/// assignment, `x = a:b` or `x := y`: its first `:` outside a `$(…)` comes before any `=` and is
/// not the start of `:=` or `::=`.
fn starts_rule(line: &str) -> bool {
    let mut depth = 0usize;
    let mut chars = line.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '$' if matches!(chars.peek(), Some((_, '(' | '{'))) => {
                chars.next();
                depth += 1;
            }
            ')' | '}' if depth > 0 => depth -= 1,
            '=' if depth == 0 => return false,
            ':' if depth == 0 => {
                let rest = line[i..].trim_start_matches(':');
                return !rest.starts_with('=');
            }
            _ => {}
        }
    }
    false
}
