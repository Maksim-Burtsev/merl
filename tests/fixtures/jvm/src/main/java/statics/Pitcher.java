package statics;

public class Pitcher {
    public static Pitcher decoct(String s) {
        return new Pitcher();
    }
    public void decoct(int n) {
    }
    public Cup pour(int n) {
        return new Cup();
    }
    public Mug pour(String s) {
        return new Mug();
    }
}
