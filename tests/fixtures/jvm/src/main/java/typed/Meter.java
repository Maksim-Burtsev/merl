package typed;

public class Meter {
    public int reading() {
        return 1;
    }

    public Gauge twin() {
        return new Gauge();
    }

    public static class Dial {
        public int tick() {
            return 1;
        }
    }
}
