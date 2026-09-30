package shop.legacy;

// #457: two enums of one name: neither is the constant's for certain.
class Door {
    Object state() { return Status.OPEN; }
    //                             ^ d: !jump
}
