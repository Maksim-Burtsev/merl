//! A payout as the bank statement prints it.

fn cents(amount: u64) -> String {
    format!("{}.{:02}", amount / 100, amount % 100)
}

pub fn entry(amount: u64) -> String {
    cents(amount)
}

pub struct Payout {
    pub amount: u64,
}

impl Payout {
    pub fn line(&self) -> String {
        entry(self.amount)
    }
}

pub struct Refund;

impl Refund {
    pub fn line(&self) -> String {
        String::from("refund")
    }
}

pub fn latest() -> Payout {
    Payout { amount: 1250 }
}

pub fn print_latest() -> String {
    let payout = latest();
    payout.line()
}
