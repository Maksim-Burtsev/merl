pub mod outer {
    pub mod weirs;
}

use outer::weirs;

pub fn net(w: weirs::Trap) -> u32 {
    w.spring()
    //^ d: src/creels/outer/weirs.rs:4
}
