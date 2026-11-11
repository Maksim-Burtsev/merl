package statics;

public class Cup {
    public static final int MAX = 3;
    public static Cup make() {
        return new Cup();
    }
    public Cup make(int n) {
        return this;
    }
    public void fill() {
    }
    public static class Size {
        public static final Size LARGE = new Size();
        public void grow() {
        }
    }
    public static final Cup SHARED = new Cup();
}
