package tagine;

// `Tagine instanceof` anywhere in a file turns the inherited field off in all of it, so Tagine
// stays the class, as before #1515.
class Brazier extends Fireside {
    void check() {
        if (Tagine instanceof Cassole) {
            Tagine.simmer();
            //     ^ d: src/main/java/tagine/Tagine.java:8; want src/main/java/tagine/Cassole.java:4 (#1515)
        }
    }
}
