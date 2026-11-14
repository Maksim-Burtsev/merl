package urnchain;

public class Urn {
    public void boil() {
    }

    public Urn size(int n) {
        return this;
    }

    public static UrnBuilder builder() {
        return new UrnBuilder();
    }
}
