package namesake.b;

import namesake.a.Pipette;

class Stillhouse {
    static class Pipette {
        static void dropper() {}
    }

    void run() {
        Pipette.dropper();
        //      ^ d: src/main/java/namesake/b/Stillhouse.java:7
    }
}

class Tapster {
    void run() {
        Pipette.dropper();
        //      ^ d: src/main/java/namesake/a/Pipette.java:4
    }
}
