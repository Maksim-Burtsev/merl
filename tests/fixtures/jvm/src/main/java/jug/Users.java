package jug;

// In each class here the name is the inherited field, of type Growler, not the class of Kit.java
// (#1515); the anonymous subclass is no enclosing type, so its class stays Unknown (#1514).
class Scullery extends Sideboard {
    void fill() {
        Jug.make().quaff();
        //         ^ d: src/main/java/jug/Kit.java:8
    }
}

class Butler {
    Object serve() {
        return new Dresser() {
            void fill() {
                Tureen.make().quaff();
                //            ^ d: picker src/main/java/jug/Kit.java:8, src/main/java/jug/Kit.java:17, src/main/java/jug/Kit.java:26, src/main/java/jug/Kit.java:35; want src/main/java/jug/Kit.java:8 (#1514)
            }
        };
    }
}

class Taproom implements Runnable,
        Hanger {
    public void run() {
        Carboy.make().quaff();
        //            ^ d: src/main/java/jug/Kit.java:8
    }
}
