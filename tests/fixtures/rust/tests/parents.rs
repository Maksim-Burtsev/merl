pub enum Verdict<T> {
    None,
    Hit(T),
}

impl<T> Verdict<T> {
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Verdict<U> {
        match self {
            Verdict::None => Verdict::None,
            Verdict::Hit(t) => Verdict::Hit(f(t)),
        }
    }
}

pub struct Rules {
    pub top: bool,
}

impl Rules {
    pub fn matched(&self, path: &str) -> Verdict<usize> {
        Verdict::Hit(path.len())
    }
}

pub struct Dir {
    pub rules: Rules,
}

pub struct Parents<'a>(Option<&'a Dir>);

impl<'a> Iterator for Parents<'a> {
    type Item = &'a Dir;

    fn next(&mut self) -> Option<&'a Dir> {
        self.0.take()
    }
}

pub struct Shelf<'a>(Option<&'a Dir>);

impl<'a> Iterator for Shelf<'a> {
    type Item = &'a Dir;

    fn next(&mut self) -> Option<&'a Dir> {
        self.0.take()
    }
}

pub struct Tally(usize);

impl Tally {
    pub fn map<F: FnOnce(usize) -> usize>(self, f: F) -> usize {
        f(self.0)
    }
}

pub struct Ledger;

impl Ledger {
    pub fn matched(&self, path: &str) -> Tally {
        Tally(path.len())
    }
}

pub struct Shelved {
    pub rules: Ledger,
}

impl Shelf<'_> {
    pub fn take(self, n: usize) -> Vec<Shelved> {
        (0..n).map(|_| Shelved { rules: Ledger }).collect()
    }
}

impl Dir {
    pub fn parents(&self) -> Parents<'_> {
        Parents(Some(self))
    }

    pub fn shelf(&self) -> Shelf<'_> {
        Shelf(Some(self))
    }

    pub fn walk(&self) {
        for dir in self.parents() {
            let _ = dir.rules.matched("a").map(|n| n + 1);
            //                             ^ d: tests/parents.rs:7
        }
        for dir in self.parents().take_while(|dir| !dir.rules.top) {
            let _ = dir.rules.matched("a").map(|n| n + 1);
            //                             ^ d: tests/parents.rs:7
        }
        for dir in self.parents().skip(1).take_while(|dir| !dir.rules.top)
        {
            let _ = dir.rules.matched("a").map(|n| n + 1);
            //                             ^ d: tests/parents.rs:7
        }
        for found in self.shelf().take(1) {
            let _ = found.rules.matched("a").map(|n| n + 1);
            //                               ^ d: !jump
        }
        for rules in self.parents().map(|dir| &dir.rules) {
            let _ = rules.matched("a").map(|n| n + 1);
            //                         ^ d: !jump
        }
    }
}
