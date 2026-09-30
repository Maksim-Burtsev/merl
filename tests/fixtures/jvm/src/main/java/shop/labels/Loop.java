package shop.labels;

// Java has no named arguments: an assignment in brackets is one.
class Loop {
    private int cached;
    int step;

    int compute() { return 1; }

    void log(int step) {}

    int count() {
        for (step = 0; step < 3; step++) {
        //   ^ d: src/main/java/shop/labels/Loop.java:6
        }
        log(step = 2);
        //  ^ d: src/main/java/shop/labels/Loop.java:6
        return (cached = compute());
        //      ^ d: src/main/java/shop/labels/Loop.java:5
    }
}
