use crate::{Mug, Pan};

// The crate's own `Any`, whose `downcast_ref` returns a `Mug` whatever the turbofish says: `get`
// is no jump to `Pan::get` (#1494 review).
pub trait Any: 'static {}

impl dyn Any {
    pub fn downcast_ref<T>(&self) -> Option<&Mug> {
        None
    }
}

pub fn own_any(a: &dyn Any) {
    a.downcast_ref::<Pan>().unwrap().get();
    //                               ^ d: !jump
}

pub fn own_if_let(a: &dyn Any) {
    if let Some(p) = a.downcast_ref::<Pan>() {
        p.get();
        //^ d: !jump
    }
}
