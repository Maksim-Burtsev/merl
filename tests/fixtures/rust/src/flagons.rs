// A link awaited in a chain of declared returns (#1469): `.await` passes an `async fn`'s
// declared return on; a plain `fn` returning a future keeps the picker.
use std::future::Future;

pub struct Flagon;
pub struct Carboy;

impl Flagon {
    pub fn tip(&self) -> u32 {
        1
    }
}

impl Carboy {
    pub fn tip(&self) -> u32 {
        2
    }
}

pub struct Cellar;

impl Cellar {
    pub async fn fetch(&self) -> Result<Flagon, ()> {
        Ok(Flagon)
    }

    pub async fn later(&self) -> Carboy {
        Carboy
    }

    pub fn soon(&self) -> impl Future<Output = Result<Carboy, ()>> {
        async { Ok(Carboy) }
    }

    pub async fn cellar(&self) -> Cellar {
        Cellar
    }
}

pub async fn serve(cellar: Cellar) -> Result<u32, ()> {
    let a = cellar.fetch().await?.tip();
    //                            ^ d: src/flagons.rs:9
    let b = cellar.later().await.tip();
    //                           ^ d: src/flagons.rs:15
    let c = cellar.cellar().await.fetch().await?.tip();
    //                                           ^ d: src/flagons.rs:9
    let d = cellar.soon().await?.tip();
    //                           ^ d: picker src/flagons.rs:9, src/flagons.rs:15
    Ok(a + b + c + d)
}
