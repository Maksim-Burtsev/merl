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
    //                                           ^ d: !jump; want tests/meshes.rs:5 (#1302)
    matcher
        .each(b"x, y", &mut caps, |caps| {
            caps.get(0).is_some()
            //   ^ d: tests/meshes.rs:5
        });
    matcher.loose(&mut caps, |caps: &Grain| caps.get(0).is_some());
    //                                           ^ d: !jump; want tests/meshes.rs:11 (#1302)
    matcher.loose(&mut caps, |grain| grain.get(0).is_some());
    //                                     ^ d: !jump
}
