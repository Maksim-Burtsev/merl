package jug;

// In each class here the name is the inherited field, of type Growler, not the class of Kit.java
// (#1515), also inside an anonymous subclass, whose base declares the field (#1514).
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
                //            ^ d: src/main/java/jug/Kit.java:8
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
