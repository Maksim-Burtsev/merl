package tagine;

// Only a value can stand left of `instanceof`, and a check without a pattern does not narrow it:
// Tagine is the inherited field, a Cassole (#1515).
class Brazier extends Fireside {
    void check() {
        if (Tagine instanceof Cassole) {
            Tagine.simmer();
            //     ^ d: src/main/java/tagine/Cassole.java:4
        }
    }
}
