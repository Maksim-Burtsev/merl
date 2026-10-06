package calls;

class Bounds {
    static final int LOW = 1;
    static final int HIGH = 2;

    public Bounds(String name) {
    }

    public Bounds(@Max(LOW < HIGH ? 1 : 2) int low, int high) {
    }
}
