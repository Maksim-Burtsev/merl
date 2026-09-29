package shop.pricing;

public final class Pricing {
    public static final int RATE_CAP = 100;

    private Pricing() {}

    public static int discount(int total) {
        return Math.min(total - 1, RATE_CAP);
    }
}
