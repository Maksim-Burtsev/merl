const std = @import("std");
const basket = @import("basket.zig");
const pricing = @import("pricing.zig");

pub fn main() void {
    const cap = pricing.RATE_CAP;
    //                  ^ d: src/pricing.zig:5
    std.debug.print("{d} {}\n", .{ cap, basket.overweight(cap) });
    //                                  ^ d: src/main.zig:2
    //                                         ^ d: src/basket.zig:36
    //                                                    ^ d: src/main.zig:6
}
