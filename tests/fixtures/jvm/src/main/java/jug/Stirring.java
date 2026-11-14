package jug;

class Stirring {
    void outside(java.io.File ladle) {
        ladle.stir();
        //    ^ d: !jump
    }

    void other(Object spoon) {
        spoon.stir();
        //    ^ d: !jump
    }
}
