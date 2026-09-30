// #469: a union tagged by an `enum(u8)` is a container, and a test's `const` is its local.
pub const Parcel = union(enum(u8)) {
    small: u32,
    large: u32,

    pub fn size(self: Parcel) u32 {
        _ = self;
        return 1;
    }
};

pub fn measure(p: Parcel) u32 {
    return p.size();
    //       ^ d: src/parcel.zig:6
}

test "size" {
    const n = 1;
    _ = n;
    //  ^ d: src/parcel.zig:18
    //   status: local
}
