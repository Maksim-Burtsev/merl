pub struct Grain;

impl Grain {
    pub fn get(&self) -> usize {
        1
    }
}

pub fn shade<Grain: Copy>(seed: Grain) -> Grain {
    let keep = |held: &Grain| held.get();
    //                             ^ d: !jump
    let _ = keep;
    seed
}
