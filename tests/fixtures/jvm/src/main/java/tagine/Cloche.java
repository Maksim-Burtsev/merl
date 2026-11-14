package tagine;

import tagine.far.Plancha;

// A private field, and a package-private one of a base in another package, are not inherited:
// Tagine stays the class (#1515).
class Cloche extends Brasero {
    void own() {
        Tagine.simmer();
        //     ^ d: src/main/java/tagine/Tagine.java:8
    }
}

class Comal extends Plancha {
    void far() {
        Tagine.simmer();
        //     ^ d: src/main/java/tagine/Tagine.java:8
    }
}
