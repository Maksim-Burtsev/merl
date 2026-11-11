package statics;

public class Mug {
    public static final int MAX = 4;
    public static Mug make() {
        return new Mug();
    }
    public void fill() {
    }
    public static class Size {
        public static final Size LARGE = new Size();
        public void grow() {
        }
    }
}
