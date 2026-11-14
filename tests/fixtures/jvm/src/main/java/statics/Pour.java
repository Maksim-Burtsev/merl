package statics;

public class Pour {
    void named() {
        Pitcher Pitcher = new Pitcher();
        Pitcher.decoct(1);
//              ^ d: !jump
    }

    void overloads(Pitcher k) {
        k.pour(1).fill();
//                ^ d: !jump
    }
}
