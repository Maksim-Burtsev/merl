// The shop of #307: every `d` case carries its answer in a comment under it.
const pricing = @import("pricing.zig");
const wh = @import("warehouse.zig");
const Tariff = pricing.Tariff;

const WEIGHT_LIMIT: u32 = 30;
comptime {
    _ = WEIGHT_LIMIT;
}

pub const Basket = struct {
    tariff: Tariff,
    coupon: pricing.Coupon,

    pub fn gross(self: Basket) u32 {
        return pricing.discount(self.tariff.rate());
        //     ^ d: src/basket.zig:2
        //             ^ d: src/pricing.zig:43
        //                                  ^ d: picker src/pricing.zig:12, src/pricing.zig:27
    }

    pub fn bonus(self: Basket) u32 {
        return self.coupon.rate() + self.gross();
        //                               ^ d: src/basket.zig:15
        //                 ^ d: picker src/pricing.zig:12, src/pricing.zig:27
    }

    pub fn label(self: Basket) []const u8 {
        _ = self.tariff.describe();
        //              ^ d: picker src/pricing.zig:18, src/pricing.zig:31
        return pricing.BANNER;
        //             ^ d: src/pricing.zig:59
    }
};

pub fn overweight(grams: u32) bool {
    const cap = WEIGHT_LIMIT + 20;
    //          ^ d: src/basket.zig:6
    return wh.weigh(grams) > cap;
    //        ^ d: src/warehouse.zig:15
    //                       ^ d: src/basket.zig:37
}

pub fn hidden(grams: u32) u32 {
    const weight = wh.weigh(grams);
    return weight + 1;
    //     ^ d: src/basket.zig:45
}

pub fn dispatch() []const u8 {
    const courier = wh.Courier.init("post");
    //                 ^ d: src/warehouse.zig:3
    //                         ^ d: src/warehouse.zig:6
    //       ^ d: src/basket.zig:51
    return courier.dispatch();
    //             ^ d: picker src/basket.zig:50, src/warehouse.zig:10
}

pub fn offer(t: Tariff) pricing.Offer {
    //          ^ d: picker src/basket.zig:4, src/pricing.zig:9
    //                              ^ d: src/pricing.zig:38
    _ = pricing.Channel.post;
    //          ^ d: src/pricing.zig:36
    //                  ^ d: none
    pricing.ledger_total += 1;
    //      ^ d: src/pricing.zig:6
    return .{ .tariff = t };
    //         ^ d: none
}

pub fn settle(total: u32) u32 {
    comptime var steps = 2;
    steps += 1;
    //^ d: src/basket.zig:72
    return pricing.gross_c(total) + wh.limit() * steps;
    //             ^ d: src/pricing.zig:51
    //                                 ^ d: src/warehouse.zig:19
}
