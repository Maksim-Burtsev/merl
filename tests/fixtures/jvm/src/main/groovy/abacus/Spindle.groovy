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

class Shuttle {
    static int pace = 0

    Shuttle(String yarn, boolean fast = pace<1, int picks = 2) {  }

    Shuttle(String yarn, String weft, String warp, String reed) {  }
}

class Bobbin {
    Bobbin(Map <String, Integer> spools, int turns = 1) {  }

    Bobbin(String a, String b, String c) {  }
}

class Reel {
    static int pace = 0

    Reel(String yarn, boolean fast = pace < 1, int picks = 2) {  }

    Reel(String yarn, String weft, String warp, String reed) {  }
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
        new Shuttle('silk', true, 3)
        //  ^ d: src/main/groovy/abacus/Spindle.groovy:27
        Map<String, Integer> spoolMap = [:]
        new Bobbin(spoolMap)
        //  ^ d: src/main/groovy/abacus/Spindle.groovy:33
        new Reel('silk', true, 3)
        //  ^ d: src/main/groovy/abacus/Spindle.groovy:41
    }
}

class Treadle {
    Treadle(@Named("cfg") Map<String, Object> m) {  }

    Treadle(String s, int n) {  }

    def fly(Map<String, Object> m) {
        new Treadle(m)
        //  ^ d: src/main/groovy/abacus/Spindle.groovy:70
    }
}

class Pirn {
    Pirn(Closure c = { -> 1 }, int b) {  }

    Pirn(String s) {  }

    def wind(Closure c) {
        new Pirn(c, 1)
        //  ^ d: src/main/groovy/abacus/Spindle.groovy:81
    }
}
