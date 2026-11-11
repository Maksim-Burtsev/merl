pub struct Caps;
pub struct Grain;

impl Caps {
    pub fn get(&self) -> usize {
        0
    }
}

impl Grain {
    pub fn get(&self) -> usize {
        1
    }
}

fn grain() -> Option<Grain> {
    Some(Grain)
}

pub fn gate() -> usize {
    let held: Caps = Caps;
    if let Some(held) = grain()
    {
        held.get();
        //   ^ d: tests/gates.rs:11
    }
    if let Some(held) =
        grain()
    {
        held.get();
        //   ^ d: tests/gates.rs:11
    }
    while let Some(held) = grain()
    {
        held.get();
        //   ^ d: tests/gates.rs:11
    }
    if held.get() > 0
    {
        held.get();
        //   ^ d: tests/gates.rs:5
    }
    match grain()
    {
        Some(held) => held.get(),
        //                 ^ d: tests/gates.rs:11
        None => held.get(),
        //           ^ d: tests/gates.rs:5
    };
    if let Some(held) = grain()
    {
        held.get();
    }
    else
    {
        held.get();
        //   ^ d: tests/gates.rs:5
    }
    if let Some(held) = grain()
    {
        held.get();
    }
    {
        held.get();
        //   ^ d: tests/gates.rs:5
    }
    held.get()
    //   ^ d: tests/gates.rs:5
}

fn caught() -> Result<Grain, Caps> {
    Ok(Grain)
}

pub fn wrappers() -> usize {
    let held: Caps = Caps;
    if let Ok(held) = caught() {
        held.get();
        //   ^ d: tests/gates.rs:11
    }
    if let Ok(held) = grain()
    {
        held.get();
        //   ^ d: !jump
    }
    if let Some(held) = caught()
    {
        held.get();
        //   ^ d: !jump
    }
    if let Some(held) = grain() && held.get() > 0
    {
        held.get();
        //   ^ d: !jump
    }
    if let Some(held) = grain() { held.get(); }
    //                                 ^ d: tests/gates.rs:11
    held.get()
    //   ^ d: tests/gates.rs:5
}

pub fn arms() -> usize {
    let held: Caps = Caps;
    let a = match caught() {
        Ok(held) => held.get(),
        //               ^ d: tests/gates.rs:11
        Err(_) => held.get(),
        //             ^ d: tests/gates.rs:5
    };
    let b = match grain() {
        Ok(held) => held.get(),
        //               ^ d: !jump
        _ => 0,
    };
    let c = match grain() {
        Some(held) if held.get() > 0 => held.get(),
        //                                   ^ d: !jump
        _ => 0,
    };
    let d = match held.get() {
        0 => match grain() {
            Some(held) => held.get(),
            //                 ^ d: tests/gates.rs:11
            None => held.get(),
            //           ^ d: tests/gates.rs:5
        },
        _ => 0,
    };
    a + b + c + d
}
