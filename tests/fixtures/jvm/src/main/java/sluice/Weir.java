package sluice;

class Trough {
    void drain() {}
}

class Gutter extends Trough {
    @Override
    void drain() {}
}

class Spigot extends Gutter {}

class Chute extends Trough { @Override void drain() {} }

class Weir extends Trough {
    @Override
    void drain() {}
    class Inner extends Gutter {
        void go() {
            drain();
        //  ^ d: src/main/java/sluice/Weir.java:9
        }
    }
    class Deeper extends Spigot {
        void go() {
            drain();
        //  ^ d: src/main/java/sluice/Weir.java:9
        }
    }
    class Plain {
        void go() {
            drain();
        //  ^ d: src/main/java/sluice/Weir.java:18
        }
    }
    Object make() {
        return new Gutter() {
            void go() {
                drain();
            //  ^ d: src/main/java/sluice/Weir.java:9
            }
        };
    }
    void use() {
        new Chute().drain();
    //              ^ d: src/main/java/sluice/Weir.java:14
    }
}
