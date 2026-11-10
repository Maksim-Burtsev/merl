package urnchain;

public class UrnBuilder {
    public UrnBuilder size(int n) {
        return this;
    }

    public Urn build() {
        return new Urn();
    }
}
