//! #377: the type of a receiver, from `self`'s `impl`, a typed parameter, `T::new()` and `-> T`.

pub struct TempDir;

impl TempDir {
    pub fn path(&self) -> &str {
        ""
    }
}

pub struct Other;

impl Other {
    pub fn path(&self) -> &str {
        ""
    }
}

pub fn tmpdir() -> TempDir {
    TempDir
}

pub fn run() -> usize {
    let td = tmpdir();
    td.path().len()
    // ^ d: src/receivers.rs:6
    // status: via tmpdir() -> TempDir
}

pub fn run2(dir: &TempDir) -> usize {
    dir.path().len()
    //  ^ d: src/receivers.rs:6
    //  status: via dir: TempDir
}

pub struct Config {
    capacity: usize,
}

pub struct Settings {
    capacity: usize,
}

pub struct Builder {
    config: Config,
    settings: Settings,
}

impl Builder {
    pub fn set(&mut self, n: usize) {
        self.config.capacity = n;
        //          ^ d: src/receivers.rs:37
        //          status: via self.config: Config
        self.settings.capacity = self.limit();
        //            ^ d: src/receivers.rs:41
        //                            ^ d: src/receivers.rs:67
        //                            status: via self: Builder
    }

    pub fn new() -> Self {
        Builder {
            config: Config { capacity: 1 },
            settings: Settings { capacity: 2 },
        }
    }

    fn limit(&self) -> usize {
        self.config.capacity
    }
}

pub fn fresh() -> usize {
    let b = Builder::new();
    b.limit() + b.settings.capacity
    //^ d: src/receivers.rs:67
    //  status: via b: Builder
    //                     ^ d: src/receivers.rs:41
    //                     status: via b.settings: Settings
}

pub fn each(dirs: Vec<TempDir>) -> usize {
    dirs.iter().map(|dir: &TempDir| dir.path().len()).sum()
    //                                  ^ d: src/receivers.rs:6
}

pub struct Holder<T> {
    item: T,
}

impl<T> Holder<T> {
    pub fn get(&self) -> &T {
        &self.item
    }
}

pub fn held(h: Holder<TempDir>) -> usize {
    h.item.path().len()
    //     ^ d: picker src/receivers.rs:6, src/receivers.rs:14, …
    //     status: chain broke at item
}

pub trait Named {
    fn name(&self) -> String {
        String::new()
    }
}

impl Named for Other {}

pub fn named(o: Other) -> String {
    o.name()
    //^ d: src/receivers.rs:103
    //  status: via o: Other
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> TempDir {
        TempDir
    }

    #[test]
    fn scratch_path() {
        let td = scratch();
        assert!(td.path().is_empty());
        //         ^ d: src/receivers.rs:6
        //         status: via scratch() -> TempDir
    }
}
