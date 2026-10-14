//! A small ledger: entries, totals and a report.
const std = @import("std");
const c = @cImport({
    @cInclude("stdio.h");
});
const mem = std.mem;
const Allocator = std.mem.Allocator;

pub const MAX_ENTRIES: usize = 1024;
const ratio: f64 = 1.5e3;
const mask = 0xff_ff;
const letter = 'z';
const newline = '\n';

/// One line of the ledger.
pub const Entry = struct {
    name: []const u8,
    amount: i64 = 0,
    tags: ?[]const []const u8 = null,

    pub fn isCredit(self: Entry) bool {
        return self.amount > 0 and self.name.len != 0;
    }
};

pub const Kind = enum(u8) {
    credit,
    debit,
    _,
};

const Value = union(enum) {
    int: i64,
    text: []const u8,
    none: void,
};

const Handle = opaque {};

pub const LedgerError = error{ Empty, Overflow } || Allocator.Error;

pub fn total(entries: []const Entry) LedgerError!i64 {
    if (entries.len == 0) return error.Empty;
    var sum: i64 = 0;
    for (entries, 0..) |entry, i| {
        _ = i;
        sum = std.math.add(i64, sum, entry.amount) catch return error.Overflow;
    }
    return sum;
}

fn describe(kind: Kind) []const u8 {
    return switch (kind) {
        .credit => "credit",
        .debit => "debit",
        else => "other",
    };
}

fn find(entries: []const Entry, name: []const u8) ?*const Entry {
    var i: usize = 0;
    outer: while (i < entries.len) : (i += 1) {
        if (!mem.eql(u8, entries[i].name, name)) continue :outer;
        return &entries[i];
    }
    return null;
}

pub fn report(allocator: Allocator, entries: []const Entry) ![]u8 {
    var list = std.ArrayList(u8).init(allocator);
    errdefer list.deinit();
    defer std.debug.print("done\n", .{});
    const writer = list.writer();
    for (entries) |entry| {
        try writer.print("{s}\t{d}\n", .{ entry.name, entry.amount });
    }
    const header =
        \\name  amount
        \\----  ------
    ;
    try writer.writeAll(header);
    return list.toOwnedSlice();
}

inline fn clamp(comptime T: type, value: T, low: T, high: T) T {
    return @max(low, @min(value, high));
}

export fn ledger_count(ptr: [*]const u8, len: usize) callconv(.C) usize {
    const bytes = ptr[0..len];
    var count: usize = 0;
    for (bytes) |b| count += @intFromBool(b == '\n');
    return count;
}

extern "c" fn puts(s: [*:0]const u8) c_int;

threadlocal var scratch: [64]u8 = undefined;

fn shifts(a: u32, b: u5) u32 {
    var x = a << b;
    x >>= 1;
    x |= a & 0x0f;
    x ^= ~a;
    x +%= 1;
    x *|= 2;
    const y = if (x > 10) x - 1 else x + 1;
    const z: ?u32 = if (y % 2 == 0) y else null;
    return (z orelse 0) / 2;
}

test "total of two entries" {
    const entries = [_]Entry{
        .{ .name = "rent", .amount = -900 },
        .{ .name = "salary", .amount = 3000, .tags = null },
    };
    try std.testing.expectEqual(@as(i64, 2100), try total(&entries));
    const blk = label: {
        break :label true;
    };
    try std.testing.expect(blk or false);
    comptime {
        std.debug.assert(MAX_ENTRIES > 0);
    }
    unreachable;
}
