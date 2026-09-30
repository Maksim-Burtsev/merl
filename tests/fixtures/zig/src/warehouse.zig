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

pub fn pack(grams: u32, boxes: u32) u32 {
    const cap = grams / boxes;
    //    ^ d: src/warehouse.zig:25
    if (cap > 1) {
        const spare = cap - 1;
        return spare + grams;
        //     ^ d: src/warehouse.zig:28
        //             ^ d: src/warehouse.zig:24
    }
    {
        const spare = 2;
        _ = spare;
        //  ^ d: src/warehouse.zig:34
    }
    return cap;
    //     ^ d: src/warehouse.zig:25
}

pub fn stack(
    crates: u32,
    height: u32,
) u32 {
    return crates * height;
    //              ^ d: src/warehouse.zig:44
    //     status: local
}
