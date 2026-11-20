package urnchain;

import java.util.List;
import java.util.Optional;
import quern.bins.TreeMap;

public class UrnShop {
    public Optional<Urn> maybe() {
        return Optional.of(new Urn());
    }

    public TreeMap<String, Urn> byName() {
        return new TreeMap<>();
    }

    public List<Urn> all() {
        return List.of();
    }

    public Optional<Urn> pick() {
        return Optional.empty();
    }

    public Optional<Skillet> pick(int n) {
        return Optional.empty();
    }
}
