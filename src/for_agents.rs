use crate::app::KEYS;
use crate::git::BASES;
use std::fmt::Write as _;

pub fn guide() -> String {
    let bases: Vec<String> = BASES.iter().map(|b| format!("`{b}`")).collect();
    let mut out = include_str!("for_agents.md").replace("{BASES}", &bases.join(", "));
    let mut group = "";
    for (key, action, g) in KEYS {
        if *g != group {
            group = g;
            _ = write!(out, "\n{g}:\n");
        }
        _ = writeln!(out, "  {key:<16} {action}");
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_guide_names_every_key_and_the_order_files() {
        let guide = super::guide();
        for (key, _, _) in super::KEYS {
            assert!(guide.contains(key), "{key} is missing");
        }
        assert!(guide.contains("merl/review/BRANCH") && guide.contains("merl/tree"));
        for base in super::BASES {
            assert!(guide.contains(&format!("`{base}`")), "{base} is missing");
        }
    }

    #[test]
    fn the_guide_tells_an_agent_to_skip_by_default() {
        let guide = super::guide();
        assert!(guide.contains("In every review you prepare, skip what the user need not read, unless they say otherwise"));
        assert!(guide.contains("Decide the order and what to skip"));
        assert!(guide.contains("\nskip tests/test_paging.py\n"));
        assert!(!guide.contains("When the user asks you to leave out"));
    }
}
