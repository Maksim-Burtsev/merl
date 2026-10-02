use crate::app::KEYS;

pub fn guide() -> String {
    let mut out = String::from(include_str!("for_agents.md"));
    let mut group = "";
    for (key, action, g) in KEYS {
        if *g != group {
            group = g;
            out += &format!("\n{g}:\n");
        }
        out += &format!("  {key:<16} {action}\n");
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
    }
}
