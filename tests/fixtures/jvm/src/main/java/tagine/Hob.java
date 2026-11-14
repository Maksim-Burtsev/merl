package tagine;

import tagine.far.Griddle;

// A protected field of a base in another package is inherited: it obscures the class Tagine (#1515).
class Hob extends Griddle {
    void far() {
        Tagine.simmer();
        //     ^ d: src/main/java/tagine/Cassole.java:4
    }
}
