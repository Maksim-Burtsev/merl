package rebound

class Workbench {
    Gimlet c = new Gimlet()
    def x = 1

    def local() { Auger c = new Auger(); c.bore() }
    //                                   ^ d: src/main/groovy/rebound/Workbench.groovy:7
    //                                     ^ d: picker src/main/groovy/rebound/Auger.groovy:4, …

    def defLocal() { def x = 2; x }
    //                          ^ d: src/main/groovy/rebound/Workbench.groovy:11

    def closureTyped() {
        [new Auger()].each { Auger c -> c.bore() }
        //                              ^ d: src/main/groovy/rebound/Workbench.groovy:15
    }

    def closureTypedLines() {
        [new Auger()].each { Auger c ->
            c.bore()
          //^ d: src/main/groovy/rebound/Workbench.groovy:20
        }
    }

    def closureVariable() {
        Closure k = { Auger c -> c.bore() }
        //                       ^ d: src/main/groovy/rebound/Workbench.groovy:27
    }

    def destructure() {
        def (a, x) = [1, 2]
        x
      //^ d: src/main/groovy/rebound/Workbench.groovy:32
    }

    def destructureOneLine() { def (a, x) = [1, 2]; x }
    //                                              ^ d: src/main/groovy/rebound/Workbench.groovy:37

    def plain() { c.bore() }
    //            ^ d: src/main/groovy/rebound/Workbench.groovy:4

    def closedBlock() { if (true) { Auger c = new Auger() }; c.bore() }
    //                                                       ^ d: src/main/groovy/rebound/Workbench.groovy:4

    def closedClosure() { [new Auger()].each { Auger c -> c.bore() }; c.bore() }
    //                                                                ^ d: src/main/groovy/rebound/Workbench.groovy:4

    def closedUntypedClosure() { [1].each { x -> x }; x }
    //                                                ^ d: src/main/groovy/rebound/Workbench.groovy:5

    def useThenDeclare() { c.bore(); Auger c = new Auger() }
    //                     ^ d: src/main/groovy/rebound/Workbench.groovy:4

    def laterLine() {
        c.bore()
      //^ d: src/main/groovy/rebound/Workbench.groovy:4
        Auger c = new Auger()
    }

    def inComment() { /* Auger c = new Auger(); */ c.bore() }
    //                                             ^ d: src/main/groovy/rebound/Workbench.groovy:4

    def inString() { def s = "Auger c = new Auger();"; c.bore() }
    //                                                 ^ d: src/main/groovy/rebound/Workbench.groovy:4

    def rightSide() { def c = c.bore() }
    //                        ^ d: src/main/groovy/rebound/Workbench.groovy:4

    def namedArgument() { foo(c: 1); c.bore() }
    //                               ^ d: src/main/groovy/rebound/Workbench.groovy:4

    def foo(Map m) { m }
}
