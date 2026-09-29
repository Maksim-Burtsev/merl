mod ledger;
mod summary;

fn main() {
    println!("{}", summary::report(&[1250, 480, 3099]));
}
