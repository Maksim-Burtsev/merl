//! A payout as the bank statement prints it.

fn cents(amount: u64) -> String {
    format!("{}.{:02}", amount / 100, amount % 100)
}

pub fn entry(amount: u64) -> String {
    cents(amount)
}
