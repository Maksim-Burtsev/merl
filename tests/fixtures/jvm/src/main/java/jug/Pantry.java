package jug;

// `this` inside an anonymous subclass is that subclass: a member it does not declare is its
// base's, not the enclosing class's (#1502).
class Cupboard {
    void wipe() {
    }
}

class Pantry {
    void wipe() {
    }

    Object open() {
        return new Cupboard() {
            void tidy() {
                this.wipe();
                //   ^ d: src/main/java/jug/Pantry.java:6
            }
        };
    }
}
