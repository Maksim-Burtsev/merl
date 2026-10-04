package abacus

import groovy.transform.CompileStatic

@CompileStatic
final class Spindle implements Comparable<Spindle> {
    private String label

    Spindle(String label, int turns = 1) {
        this.label = label
    }

    Spindle() {  }

    int compareTo(Spindle other) { 0 }
}

class Heddle {
    Heddle(String eye) {  }

    Heddle(Map<String, Object> opts) {  }
}

class Loom {
    def weave() {
        Spindle warp = new Spindle('warp')
      //^ d: src/main/groovy/abacus/Spindle.groovy:6
        //                 ^ d: src/main/groovy/abacus/Spindle.groovy:9
        warp <=> new Spindle('weft', 3) <=> new Spindle()
        //           ^ d: src/main/groovy/abacus/Spindle.groovy:9
        //                                      ^ d: src/main/groovy/abacus/Spindle.groovy:13
        new Heddle([eye: 'warp']) <=> new Heddle(eye: 'weft')
        //  ^ d: src/main/groovy/abacus/Spindle.groovy:21
        //                                ^ d: src/main/groovy/abacus/Spindle.groovy:21
        new Heddle(eye: 'weft', lift: 2)
        //  ^ d: src/main/groovy/abacus/Spindle.groovy:21
    }
}

class Shuttle {
    Shuttle(@Named("cfg") Map<String, Object> m) {  }

    Shuttle(String s, int n) {  }

    def fly(Map<String, Object> m) {
        new Shuttle(m)
        //  ^ d: src/main/groovy/abacus/Spindle.groovy:41
    }
}

class Bobbin {
    Bobbin(Closure c = { -> 1 }, int b) {  }

    Bobbin(String s) {  }

    def wind(Closure c) {
        new Bobbin(c, 1)
        //  ^ d: src/main/groovy/abacus/Spindle.groovy:52
    }
}
