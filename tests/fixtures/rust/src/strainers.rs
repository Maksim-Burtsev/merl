use crate::servings::Crock;

// A place declared `Option<Crock>` or `Result<Crock, _>`, unwrapped by `if let`, `match` or
// `let … else`, lands on `Crock`'s method, not `Wok`'s namesake (#1494).
pub fn strain(n: Option<Crock>, r: Result<Crock, String>) {
    if let Some(c) = &n {
        c.stew();
        //^ d: src/servings.rs:5
    }
    match &n {
        Some(d) => {
            d.stew();
            //^ d: src/servings.rs:5
        }
        None => {}
    }
    let Some(e) = &n else { return };
    e.stew();
    //^ d: src/servings.rs:5
    let Ok(f) = &r else { return };
    f.stew();
    //^ d: src/servings.rs:5
}
