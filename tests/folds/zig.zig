// Every construct `f` folds in Zig, and the cases that once broke it.
const std = @import("std");  // f: none
const Allocator = std.mem.Allocator;  // f: none

pub const Color = enum(u8) {  // f: 5-8
    red,  // f: 5-8
    green,  // f: 5-8
};  // f: 5-8

const Shape = union(enum) {  // f: 10-15
    circle: f32,  // f: 10-15
    square: struct {  // f: 12-14
        side: f32,  // f: 12-14
    },  // f: 12-14
};  // f: 10-15

const Handle = opaque {  // f: 17-19
    pub fn close(_: *Handle) void {}  // f: 17-19
};  // f: 17-19

pub const Error = error{  // f: 21-24
    Closed,  // f: 21-24
    Broken,  // f: 21-24
};  // f: 21-24

pub const Point = packed struct(u32) {  // f: 26-33
    x: u16 = 0,  // f: 26-33
    y: u16 = 0,  // f: 26-33

    pub fn sum(self: Point) u32 {  // f: 30-32
        return @as(u32, self.x) + self.y;  // f: 30-32
    }  // f: 30-32
};  // f: 26-33

const Callback = *const fn (  // f: 35-38
    ctx: *anyopaque,  // f: 35-38
    n: usize,  // f: 35-38
) callconv(.c) void;  // f: 35-38

const banner =  // f: none
    \\fn not_code() {  // f: 41-44
    \\    "}" ' {  // f: 41-44
    // a comment between the lines  // f: 41-44
    \\}  // f: 41-44
;  // f: none

pub fn @"quoted name"(  // f: 47-49
    gpa: Allocator,  // f: 47-49
    items: []const u8,  // f: 47-49
) Error!usize {  // f: 50-105
    var total: usize = 0;  // f: 50-105
    const brace = '{';  // f: 50-105
    const quote = '\'';  // f: 50-105
    const text = "} { // not a comment";  // f: 50-105
    _ = .{ brace, quote, text };  // f: 50-105
    for (items, 0..) |item, i| {  // f: 56-59
        if (item == '}') continue;  // f: 50-105
        total += i;  // f: 50-105
    }  // f: 50-105
    var i: usize = 0;  // f: 50-105
    while (i < items.len) : (i += 1) {  // f: 61-62
        total += items[i];  // f: 50-105
    } else {  // f: 63-65
        total += 1;  // f: 50-105
    }  // f: 50-105
    if (total > 10) {  // f: 66-67
        total = 10;  // f: 50-105
    } else if (total > 5)  // f: 68-72
        total = 5  // f: 50-105
    else {  // f: 70-72
        total = 0;  // f: 50-105
    }  // f: 50-105
    if (total == 0)  // f: 73-74
        return error.Broken;  // f: 50-105
    const small = if (total < 3)  // f: 75-78
        total  // f: 50-105
    else  // f: 50-105
        3;  // f: 50-105
    const list = try std.ArrayList(u8).initCapacity(  // f: 79-82
        gpa,  // f: 50-105
        small,  // f: 50-105
    );  // f: 50-105
    _ = list;  // f: 50-105
    const kind = switch (total) {  // f: 84-91
        0 => "none",  // f: 50-105
        1, 2 => blk: {  // f: 86-89
            const x = "two";  // f: 50-105
            break :blk x;  // f: 50-105
        },  // f: 50-105
        else => "many",  // f: 50-105
    };  // f: 50-105
    _ = kind;  // f: 50-105
    const chained = std.mem  // f: 93-94
        .trim(u8, items, " ")  // f: 50-105
        .len;  // f: 50-105
    _ = chained;  // f: 50-105
    _ = @min(  // f: 50-105
        total,  // f: 50-105
        small,  // f: 50-105
    );  // f: 50-105
    defer {  // f: 101-103
        total = 0;  // f: 50-105
    }  // f: 50-105
    return total;  // f: 50-105
}  // f: 50-105

fn spin() void {  // f: 107-119
    asm volatile (  // f: 108-110
        \\nop  // f: 107-119
        ::: "memory");  // f: 107-119
    outer: for (0..3) |n| {  // f: 111-115
        inline for (.{ 1, 2 }) |m| {  // f: 112-114
            if (n == m) continue :outer;  // f: 107-119
        }  // f: 107-119
    }  // f: 107-119
    comptime {  // f: 116-118
        std.debug.assert(true);  // f: 107-119
    }  // f: 107-119
}  // f: 107-119

test "quoted name" {  // f: 121-126
    try std.testing.expectEqual(  // f: 122-125
        @as(usize, 0),  // f: 121-126
        try @"quoted name"(std.testing.allocator, ""),  // f: 121-126
    );  // f: 121-126
}  // f: 121-126
