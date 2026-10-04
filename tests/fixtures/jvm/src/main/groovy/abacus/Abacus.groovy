package abacus

import groovy.transform.CompileStatic

trait Countable {
    abstract BigDecimal tallyUp(List<BigDecimal> xs)
}

abstract class Rack {
    String motto = 'beads'
}

enum Tint {
    AMBER, JADE
}

@CompileStatic class Bead {
    int size
}

class Abacus extends Rack implements Countable {
//                   ^ d: src/main/groovy/abacus/Abacus.groovy:9
//                                   ^ d: src/main/groovy/abacus/Abacus.groovy:5
    static final String NOTES = '''
        def ghost() {
        class Phantom {
    '''
    static final String SLASHY = $/
        def ghost() {
        class Phantom {
    /$

    BigDecimal tallyUp(List<BigDecimal> xs) { xs.sum() }
    def reckon() {
        def total = 0
        total += Till.unlock()
      //^ d: src/main/groovy/abacus/Abacus.groovy:35
        //            ^ d: src/main/java/abacus/Till.java:4
        tallyUp([]) + total + motto.size()
      //^ d: src/main/groovy/abacus/Abacus.groovy:33
        //                    ^ d: src/main/groovy/abacus/Abacus.groovy:10
    }
    static def helper(Tint t) {
    //                ^ d: src/main/groovy/abacus/Abacus.groovy:13
        new Bead()
        //  ^ d: src/main/groovy/abacus/Abacus.groovy:17
    }
    private def loader() {
        def hail = { name -> "hi $name" }
        hail('x') + helper(Tint.AMBER) + release
      //^ d: src/main/groovy/abacus/Abacus.groovy:49
        //          ^ d: src/main/groovy/abacus/Abacus.groovy:43
        //                               ^ d: src/main/groovy/abacus/Abacus.groovy:55
    }
    def release = '1.0'
    def sweep() {
        ghost() + Phantom
      //^ d: none
        //        ^ d: none
        def m = [quirk: 'x']
        m.wobble = 2
        m.quirk + m.wobble
        //^ d: none
        //          ^ d: none
        reckon() + loader()
      //^ d: src/main/groovy/abacus/Abacus.groovy:34
        //         ^ d: src/main/groovy/abacus/Abacus.groovy:48
    }
}
