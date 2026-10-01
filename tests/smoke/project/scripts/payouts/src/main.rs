mod ledger;
mod summary;

fn main() {
    println!("{}", summary::report(&[1250, 480, 3099]));
}

pub fn statement(amount: u64) -> String {
    use crate::ledger::entry;
    entry(amount)
}
