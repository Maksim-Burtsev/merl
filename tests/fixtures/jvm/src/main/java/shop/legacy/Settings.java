package shop.legacy;

public class Settings {
    public static class Tariff {}

    private static String owner() {
        return "legacy";
    }
}

enum TimeUnit {
    MILLISECONDS,
    SECONDS;

    static TimeUnit slow() {
        return TimeUnit.SECONDS;
        //              ^ d: src/main/java/shop/legacy/Settings.java:13
    }
}
