package quern;

public class Grist {
    private final int weight;

    private Grist(int weight) {
        this.weight = weight;
    }

    public int weight() {
        return weight;
    }

    public static Builder builder() {
        return new Builder();
    }

    public static class Builder {
        private int weight;

        public Builder coarse(int level) {
            return this;
        }

        public Builder weight(int grams) {
            this.weight = grams;
            return this;
        }

        public Grist build() {
            return new Grist(weight);
        }
    }
}
