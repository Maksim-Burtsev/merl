//! One entry per payout, then the sum.

fn cents(amount: u64) -> String {
    format!("{} EUR", amount as f64 / 100.0)
}

pub fn report(amounts: &[u64]) -> String {
    let sum: u64 = amounts.iter().sum();
    let mut out: Vec<String> = amounts.iter().map(|&a| crate::ledger::entry(a)).collect();
    out.push(cents(sum));
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u64> {
        vec![100, 250]
    }

    #[test]
    fn sums() {
        assert!(report(&sample()).ends_with(&cents(350)));
    }
}
