use crate::pricing::{cents, discount, Bits, Coupon, Money, Offer, Priced, Tariff, CURRENCY};
use crate::warehouse::{self, Courier};

const WEIGHT_LIMIT: u32 = 30;

pub struct Basket {
    tariff: Tariff,
    coupon: Coupon,
    pub owner: String,
}

impl Basket {
    pub fn new(tariff: Tariff) -> Self {
        Basket {
        // ^ d: src/basket.rs:6
            tariff,
            coupon: Coupon,
            // ^ d: none; want src/basket.rs:8 (#316)
            //      ^ d: src/pricing.rs:21
            owner: String::new(),
        }
    }

    pub fn gross(&self) -> Money {
        //                 ^ d: src/pricing.rs:41
        discount(self.tariff.rate())
        // ^ d: src/pricing.rs:55
        //            ^ d: none; want src/basket.rs:7 (#370)
        //                   ^ d: picker src/pricing.rs:13, src/pricing.rs:24; want src/pricing.rs:13 (#377)
    }

    pub fn bonus(&self) -> u32 {
        self.coupon.rate() + self.gross()
        //          ^ d: picker src/pricing.rs:13, src/pricing.rs:24; want src/pricing.rs:24 (#377)
        //                        ^ d: src/basket.rs:24
    }
}

impl Priced for Basket {
    //  ^ d: src/pricing.rs:32
    fn price(&self) -> u32 {
        self.bonus()
    }
}

pub fn describe_any(t: &Tariff, c: &Coupon) -> String {
    t.describe() + &c.describe()
    //^ d: picker src/pricing.rs:16, src/pricing.rs:27; want src/pricing.rs:16 (#377)
    //                ^ d: picker src/pricing.rs:16, src/pricing.rs:27; want src/pricing.rs:27 (#377)
}

pub fn restock(discount: u32) -> u32 {
    discount + WEIGHT_LIMIT
    // ^ d: src/pricing.rs:55; want src/basket.rs:52 (#353)
    //         ^ d: src/basket.rs:4
}

pub fn overweight(grams: u32) -> bool {
    let limit = WEIGHT_LIMIT + 20;
    warehouse::weigh(grams) > limit
    //         ^ d: picker src/basket.rs:111, src/warehouse.rs:23; want src/warehouse.rs:23 (#350)
    //                        ^ d: src/basket.rs:59
}

pub fn dispatch() -> String {
    let courier = Courier::new("post");
    //            ^ d: src/warehouse.rs:10
    //                     ^ d: src/warehouse.rs:15
    courier.name
    //      ^ d: none; want src/warehouse.rs:11 (#370)
}

pub fn first(v: Option<Courier>) -> Courier {
    v.unwrap()
    //^ d: src/warehouse.rs:18; want !jump (#358)
}

pub fn offer(o: Offer) -> u32 {
    match o {
        Offer::Plain => 0,
        //     ^ d: none; want src/pricing.rs:37 (#370)
        Offer::Cut(n) => n,
    }
}

pub fn depot() -> bool {
    warehouse::depot::open()
    //         ^ d: picker src/basket.rs:86, src/warehouse.rs:27; want src/warehouse.rs:27 (#350)
    //                ^ d: src/warehouse.rs:28
}

pub fn money() -> u32 {
    let bits = Bits { whole: cents!(3) };
    //         ^ d: src/pricing.rs:43
    //                       ^ d: src/pricing.rs:48
    unsafe { bits.whole + CURRENCY.len() as u32 }
    //                    ^ d: src/pricing.rs:6
}

pub async fn pay(total: u32) -> u32 {
    crate::pricing::settle(total).await
    //              ^ d: src/pricing.rs:59
}

#[derive(Debug)]
//       ^ d: src/pricing.rs:63; want none (#370)
pub struct Receipt;

pub fn hidden(o: Offer, grams: u32) -> u32 {
    //           ^ d: src/pricing.rs:36
    let weigh = warehouse::weigh(grams);
    weigh + offer(o)
    // ^ d: picker src/basket.rs:111, src/warehouse.rs:23; want src/basket.rs:111 (#353)
}
