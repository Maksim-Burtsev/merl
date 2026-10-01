package calls;

enum Mode {
    A, B;

    static Mode of(String name) {
        for (var k : values()) {
        //           ^ d: src/main/java/calls/Mode.java:3
            if (k.name().equals(name)) return k;
        }
        return A;
    }
}
