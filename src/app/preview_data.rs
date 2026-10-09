use std::hash::{Hash, Hasher};
use std::thread::JoinHandle;

use super::*;
use crate::markdown::data::{self, Parsed};

type Job = JoinHandle<Result<Parsed, String>>;

#[derive(Default)]
pub(super) struct Parse {
    job: Option<(PathBuf, u64, Job)>,
    pub(super) done: Option<(PathBuf, u64, Parsed)>,
}

pub(super) fn lines_hash(lines: &[String]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    lines.hash(&mut h);
    h.finish()
}

pub(super) fn no_preview(path: &Path) -> String {
    match path.extension() {
        Some(ext) => format!("no preview for .{}", ext.to_string_lossy()),
        None => {
            let name = path.file_name().unwrap_or(path.as_os_str());
            format!("no preview for {}", name.to_string_lossy())
        }
    }
}

impl App {
    pub(super) fn parse_ready(&self, path: &Path) -> bool {
        !data::parses(path) || self.parse.done.as_ref().is_some_and(|d| d.0 == path)
    }

    pub(crate) fn preview_pending(&self) -> bool {
        let path = self.buf.path.as_deref();
        path.is_some_and(|p| self.previewed.contains(p) && !self.parse_ready(p))
    }

    pub(crate) fn preview_fresh(&mut self) {
        if self.previewing() && self.buf.path.as_deref().is_some_and(data::parses) {
            self.parse_stale(lines_hash(&self.buf.lines));
        }
    }

    pub(super) fn parse_toggle(&mut self, path: &Path) -> bool {
        if self.previewed.contains(path) {
            if self.parse_ready(path) {
                return false;
            }
            self.previewed.remove(path);
            self.parse.job = None;
            return true;
        }
        if !data::parses(path) {
            return false;
        }
        let hash = lines_hash(&self.buf.lines);
        if self
            .parse
            .done
            .as_ref()
            .is_some_and(|d| d.0 == path && d.1 == hash)
        {
            return false;
        }
        self.previewed.insert(path.to_path_buf());
        self.parse_start(hash);
        true
    }

    pub(super) fn parse_stale(&mut self, hash: u64) -> bool {
        let path = self.buf.path.as_deref();
        let done = self.parse.done.as_ref();
        let stale = done.is_some_and(|d| Some(d.0.as_path()) == path && d.1 != hash);
        if stale {
            self.parse.done = None;
            self.parse_start(hash);
        }
        stale
    }

    fn parse_start(&mut self, hash: u64) {
        let Some(path) = self.buf.path.clone() else {
            return;
        };
        let Some(kind) = data::data(&path) else {
            return;
        };
        if self
            .parse
            .job
            .as_ref()
            .is_some_and(|j| j.0 == path && j.1 == hash)
        {
            return;
        }
        let lines = self.buf.lines.clone();
        let spawned = std::thread::Builder::new()
            .name(data::THREAD.into())
            .spawn(move || data::parse(kind, &lines));
        match spawned {
            Ok(job) => self.parse.job = Some((path, hash, job)),
            Err(_) => {
                self.previewed.remove(&path);
                self.message = "p: could not parse".into();
            }
        }
    }

    pub(super) fn parse_tick(&mut self) -> bool {
        let Some(path) = self.buf.path.clone() else {
            return false;
        };
        if !self.parse.job.as_ref().is_some_and(|j| j.2.is_finished()) {
            if self.parse.job.is_none()
                && self.previewed.contains(&path)
                && !self.parse_ready(&path)
            {
                self.parse_start(lines_hash(&self.buf.lines));
                return true;
            }
            return false;
        }
        let Some((from, hash, job)) = self.parse.job.take() else {
            return false;
        };
        if from != path {
            return false;
        }
        match job.join() {
            Ok(Ok(parsed)) => {
                self.parse.done = Some((from, hash, parsed));
                if self.previewed.contains(&path) {
                    self.preview_open();
                }
            }
            Ok(Err(why)) => {
                self.previewed.remove(&path);
                self.message = format!("p: {why}");
            }
            Err(_) => {
                self.previewed.remove(&path);
                self.message = "p: could not parse".into();
            }
        }
        true
    }

    #[cfg(test)]
    pub(crate) fn settle_parse(&mut self) {
        while self.parse.job.as_ref().is_some_and(|j| !j.2.is_finished()) {
            std::thread::yield_now();
        }
        self.parse_tick();
    }
}
