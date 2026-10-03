package typed;

public class Gauge {
    public int reading() {
        return 2;
    }

    public static class Dial {
        public int tick() {
            return 2;
        }
    }
}
