use super::{App, Focus, git};

impl App {
    pub(super) fn blame_line(&mut self) {
        if self.focus != Focus::Code || !self.flush() {
            return;
        }
        let at_base = self.base_place();
        let (rev, path, line1) = match (at_base, self.review.as_ref()) {
            (Some(p), Some(r)) => (Some(r.merge_base.clone()), p.path, p.line1),
            _ => match self.rel_current() {
                Some(rel) => (None, rel, self.line + 1),
                None => return,
            },
        };
        let range = format!("{line1},{line1}");
        let path = path.to_string_lossy().into_owned();
        let mut args = vec!["blame", "--porcelain", "-L", &range];
        if let Some(rev) = rev.as_deref() {
            args.push(rev);
        }
        args.extend(["--", &path]);
        let Ok(out) = git::git(&self.root, &args) else {
            self.message = "not in git".into();
            return;
        };
        let hash = out.split(' ').next().unwrap_or_default();
        if hash.bytes().all(|b| b == b'0') {
            self.message = "not committed".into();
            return;
        }
        self.message = git::git(
            &self.root,
            &["show", "-s", "--format=%h · %as · %an · %s", hash],
        )
        .unwrap_or_default();
    }
}
