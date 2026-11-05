pub struct Caps;
pub struct Grain;

impl Caps {
    pub fn get(&self, i: usize) -> Option<usize> {
        Some(i)
    }
}

impl Grain {
    pub fn get(&self, i: usize) -> Option<usize> {
        Some(i)
    }
}

pub trait Sifter {
    type Captures;

    fn each<F>(&self, haystack: &[u8], caps: &mut Self::Captures, mut matched: F) -> bool
    where
        F: FnMut(&Self::Captures) -> bool,
    {
        haystack.is_empty() || matched(caps)
    }

    fn loose<F>(&self, caps: &mut Self::Captures, keep: F) -> F {
        let _ = caps;
        keep
    }
}

pub struct Mesh;

impl Sifter for Mesh {
    type Captures = Caps;
}

fn mesh() -> Mesh {
    Mesh
}

pub fn sift() {
    let matcher = mesh();
    let mut caps = Caps;
    matcher.each(b"a, b", &mut caps, |caps| caps.get(1).is_some());
    //                                           ^ d: tests/meshes.rs:5
    matcher
        .each(b"x, y", &mut caps, |caps| {
            caps.get(0).is_some()
            //   ^ d: tests/meshes.rs:5
        });
    matcher.loose(&mut caps, |caps: &Grain| caps.get(0).is_some());
    //                                           ^ d: tests/meshes.rs:11
    matcher.loose(&mut caps, |grain| grain.get(0).is_some());
    //                                     ^ d: !jump
    let _ = (matcher.loose(&mut caps, |caps: &Grain| caps.get(0)), caps.get(3));
    //                                                    ^ d: tests/meshes.rs:11
    //                                                                  ^ d: !jump
    let peek = |caps: &Grain| caps.get(0);
    //                             ^ d: tests/meshes.rs:11
    matcher.each(b"z", &mut caps, |grain| caps.get(4).is_some());
    //                                         ^ d: !jump
    matcher.each(b"w", &mut caps, |caps| caps.get(0).is_none() || caps.get(5).is_some());
    //                                                                 ^ d: tests/meshes.rs:5
    matcher.each(b"r", &mut caps, |caps| { let caps: Grain = Grain; caps.get(6).is_some() });
    //                                                                   ^ d: !jump
    matcher.loose(&mut caps, |caps: &Grain| { let caps = mesh(); caps.get(7) });
    //                                                                ^ d: !jump
    let grain: Grain = Grain;
    matcher.loose(&mut caps, |grain: &Grain| { let grain = mesh(); grain.get(8) });
    //                                                                   ^ d: !jump
    let (_, sieve) = (matcher.loose(&mut caps, |sieve: &Grain| sieve.get(9)), Caps);
    sieve.get(10);
    //    ^ d: !jump
    let _ = (peek, grain);
}
