pub struct LockPackage {
    pub registry_dir: String,
    pub name: String,
}
pub struct CargoReach {
    pub reached: Vec<String>,
    pub every_locked: Vec<LockPackage>,
}
pub fn cargo_reach(lock: &str, from: &str) -> Option<CargoReach> {
    let mut packages: Vec<(String, String, Vec<String>)> = Vec::new();
    let mut in_deps = false;
    for l in lock.lines() {
        let t = l.trim();
        if t == "[[package]]" {
            packages.push(Default::default());
            in_deps = false;
            continue;
        }
        let Some(p) = packages.last_mut() else {
            continue;
        };
        if let Some(v) = t.strip_prefix("name = ") {
            v.trim_matches('"').clone_into(&mut p.0);
        } else if let Some(v) = t.strip_prefix("version = ") {
            v.trim_matches('"').clone_into(&mut p.1);
        } else if let Some(v) = t.strip_prefix("dependencies = [") {
            in_deps = !v.contains(']');
            p.2.extend(quoted_names(v));
        } else if in_deps {
            in_deps = !t.starts_with(']');
            p.2.extend(quoted_names(t));
        } else if t.starts_with('[') {
            in_deps = false;
        }
    }
    packages.iter().find(|p| p.0 == from)?;
    let mut reached = vec![from.to_owned()];
    let mut i = 0;
    while i < reached.len() {
        let name = reached[i].clone();
        for p in packages.iter().filter(|p| p.0 == name) {
            for d in &p.2 {
                if !reached.contains(d) {
                    reached.push(d.clone());
                }
            }
        }
        i += 1;
    }
    let every_locked = packages
        .into_iter()
        .map(|(n, v, _)| LockPackage {
            registry_dir: format!("{n}-{v}"),
            name: n,
        })
        .collect();
    Some(CargoReach {
        reached,
        every_locked,
    })
}
fn quoted_names(s: &str) -> impl Iterator<Item = String> + '_ {
    s.split('"')
        .skip(1)
        .step_by(2)
        .filter_map(|q| q.split_whitespace().next().map(str::to_owned))
}
pub fn cargo_package_name(toml: &str) -> Option<String> {
    let mut in_package = false;
    for l in toml.lines() {
        let t = l.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
        } else if in_package && let Some(v) = t.strip_prefix("name") {
            let v = v.trim_start().strip_prefix('=')?;
            return Some(v.trim().trim_matches('"').to_owned());
        }
    }
    None
}
