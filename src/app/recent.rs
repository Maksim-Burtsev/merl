use std::time::{SystemTime, UNIX_EPOCH};

use super::*;
use crate::picker::Lead;

const CAP: usize = 200;
const HALF_LIFE: f64 = 30.0 * 86400.0;

fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

pub(super) fn mode() -> String {
    std::env::var("MERL_406").unwrap_or_default()
}

impl App {
    fn recent_store(&self) -> Option<PathBuf> {
        git::dirs(&self.root).map(|(git_dir, _)| git_dir.join("merl/recent"))
    }

    pub(super) fn read_recent(&self) -> Vec<(PathBuf, f64)> {
        let Some(text) = self.recent_store().and_then(|s| std::fs::read_to_string(s).ok()) else {
            return Vec::new();
        };
        let t = now();
        text.lines()
            .filter_map(|l| {
                let mut f = l.splitn(3, '\t');
                let score: f64 = f.next()?.parse().ok()?;
                let at: f64 = f.next()?.parse().ok()?;
                let decayed = score * 0.5f64.powf((t - at) / HALF_LIFE);
                Some((PathBuf::from(f.next()?), decayed))
            })
            .collect()
    }

    pub(super) fn note_recent(&mut self, path: &Path) {
        let (Some(store), Ok(rel)) = (self.recent_store(), path.strip_prefix(&self.root)) else {
            return;
        };
        let mut list = self.read_recent();
        let score = list.iter().find(|(p, _)| p == rel).map_or(0.0, |e| e.1) + 1.0;
        list.retain(|(p, _)| p != rel);
        list.insert(0, (rel.to_path_buf(), score));
        list.truncate(CAP);
        let t = now();
        let text: String = list
            .iter()
            .map(|(p, s)| format!("{s}\t{t}\t{}\n", p.display()))
            .collect();
        let _ = std::fs::create_dir_all(store.parent().unwrap());
        let _ = std::fs::write(store, text);
    }

    pub(super) fn lead_406(&self, listed: &[&PathBuf]) -> Lead {
        let at: HashMap<&Path, u32> = (listed.iter().enumerate())
            .map(|(i, p)| (p.as_path(), i as u32))
            .collect();
        let recent = self.read_recent();
        let idx = |p: &PathBuf| at.get(p.as_path()).copied();
        match mode().as_str() {
            "vscode" => {
                let current = self.buf.path.as_ref().and_then(|p| p.strip_prefix(&self.root).ok());
                let first = recent.first().is_some_and(|(p, _)| Some(p.as_path()) == current);
                Lead::VsCode(recent.iter().filter_map(|(p, _)| idx(p)).collect(), first)
            }
            "smart" => {
                let mut r: Vec<(u32, f64)> =
                    recent.iter().filter_map(|(p, s)| Some((idx(p)?, *s))).collect();
                r.sort_by(|a, b| b.1.total_cmp(&a.1));
                Lead::Smart(r.into_iter().map(|(i, s)| (i, (8.0 * (1.0 - 1.0 / (1.0 + s))).round() as u32)).collect())
            }
            _ => Lead::None,
        }
    }

    pub fn open_recent_picker(&mut self) {
        let current = self.buf.path.as_ref().and_then(|p| p.strip_prefix(&self.root).ok());
        let items: Vec<PickItem> = self
            .read_recent()
            .into_iter()
            .filter(|(p, _)| Some(p.as_path()) != current && self.files.contains(p))
            .map(|(p, _)| PickItem {
                label: p.display().to_string(),
                path: p,
                line: 0,
                col: 0,
                code_at: None,
                deleted: false,
            })
            .collect();
        let mut picker = Picker::new("Recent", items, true);
        picker.settle();
        self.picker = Some(picker);
        self.mode = Mode::Picker(PickerKind::Files);
    }
}
