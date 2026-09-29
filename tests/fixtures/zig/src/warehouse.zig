const pricing = @import("pricing.zig");

pub const Courier = struct {
    name: []const u8,

    pub fn init(name: []const u8) Courier {
        return .{ .name = name };
    }

    pub fn dispatch(self: Courier) []const u8 {
        return self.name;
    }
};

pub fn weigh(grams: u32) u32 {
    return grams / 1000;
}

pub fn limit() u32 {
    const cap = pricing.RATE_CAP * 2;
    return cap;
}
