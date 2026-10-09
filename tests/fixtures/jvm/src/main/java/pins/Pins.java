package pins;

class Pins {
    void counts(Shim shim, int a, int b) {
        shim.fit("""
        //   ^ d: src/main/java/pins/Shim.java:6
            a)
            """, 2);
        shim.fit(',');
        //   ^ d: src/main/java/pins/Shim.java:4
        shim.fit(a < b ? "x" : "y", 2);
        //   ^ d: picker src/main/java/pins/Shim.java:4, src/main/java/pins/Shim.java:6
    }

    <T> void untyped(T t) {
        int stock = 1;
        t.stock = stock;
        //^ d: picker src/main/java/pins/Shim.java:8
        print(early);
        //    ^ d: none
        int early = 2;
        t.slot();
        //^ d: src/main/java/pins/Ticket.java:3
    }

    void block(Shim shim) {
        shim.fit("""
        //   ^ d: src/main/java/pins/Shim.java:4
            text
            """);
    }
}
