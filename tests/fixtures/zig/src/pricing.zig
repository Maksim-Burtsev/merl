//! The shop of #307: every `d` case carries its answer in a comment under it.
//! A doc comment that reads like code declares nothing:
//! pub fn weigh(grams: u32) u32 {

pub const RATE_CAP: u32 = 100;
pub var ledger_total: u32 = 0;
threadlocal var last_rate: u32 = 0;

pub const Tariff = struct {
    base: u32,

    pub fn rate(self: Tariff) u32 {
        last_rate = self.base;
        // ^ d: src/pricing.zig:7
        return self.base;
    }

    pub fn describe(self: Tariff) []const u8 {
        _ = self;
        return "tariff";
    }
};

pub const Coupon = struct {
    percent: u32,

    pub fn rate(self: Coupon) u32 {
        return self.percent;
    }

    pub fn describe(_: Coupon) []const u8 {
        return "coupon";
    }
};

pub const Channel = enum { post, courier };

pub const Offer = union(enum) {
    tariff: Tariff,
    coupon: Coupon,
};

pub inline fn discount(total: u32) u32 {
    return @min(total, RATE_CAP) - 1;
}

noinline fn round(total: u32) u32 {
    return total / 5 * 5;
}

export fn gross_c(total: u32) u32 {
    return round(discount(total)) + @as(u32, @intCast(abs(0)));
    //     ^ d: src/pricing.zig:47
    //                                                ^ d: src/pricing.zig:57
}

extern "c" fn abs(x: c_int) c_int;

pub const BANNER =
    \\# Pricing
    \\```zig
    \\pub fn discount(total: u32) u32 {
    \\const Coupon = struct {
    \\```
;

test "discount caps at RATE_CAP" {
    //   ^ d: src/pricing.zig:43
    try @import("std").testing.expect(discount(500) == RATE_CAP - 1);
    //                                                  ^ d: src/pricing.zig:5
}
