/* One struct with two bodies in two headers: which one a `struct twin` is, is not read. */
struct twin {
    struct seal *seal;
};
