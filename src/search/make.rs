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
pub fn make_recipe_command(text: &str, line1: usize) -> Option<usize> {
    let mut in_rule = false;
    let mut after_backslash = None;
    for (i, l) in text.lines().take(line1).enumerate() {
        let recipe = match after_backslash {
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
        if i + 1 == line1 {
            return recipe;
        }
        after_backslash = l.ends_with('\\').then_some(recipe);
    }
    None
}
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
