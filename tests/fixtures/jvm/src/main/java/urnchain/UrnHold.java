package urnchain;

public class UrnHold<Urn> {
    void pour(Urn[] arr) {
        arr[0].boil();
        //     ^ d: !jump
    }
}
