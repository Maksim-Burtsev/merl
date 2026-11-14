package rebound

class Sawhorse {
    Gimlet c = new Gimlet()
    def x = 1

    def twoDefs() { def a = 1, x = 2; x }
    //                                ^ d: src/main/groovy/rebound/Sawhorse.groovy:7

    def twoDefsLines() {
        def a = 1, x = 2
        x
      //^ d: src/main/groovy/rebound/Sawhorse.groovy:11
    }

    def noInitializer() {
        Auger c
        c = new Auger()
        c.bore()
      //^ d: src/main/groovy/rebound/Sawhorse.groovy:17
    }

    def defaultParameter() {
        def f = { Auger c = new Auger() -> c.bore() }
        //                                 ^ d: src/main/groovy/rebound/Sawhorse.groovy:24
    }

    def headerLines() {
        [a: new Auger()].each { String s,
                                Auger c ->
            c.bore()
          //^ d: src/main/groovy/rebound/Sawhorse.groovy:30
        }
    }

    def callInInitializer() { def a = foo(1, x); x }
    //                                       ^ d: src/main/groovy/rebound/Sawhorse.groovy:5
    //                                           ^ d: src/main/groovy/rebound/Sawhorse.groovy:5

    def noInitializerClosed() {
        if (true) {
            Auger c
        }
        c.bore()
      //^ d: src/main/groovy/rebound/Sawhorse.groovy:4
    }

    def defaultParameterClosed() { def f = { Auger c = new Auger() -> c }; c.bore() }
    //                                                                     ^ d: src/main/groovy/rebound/Sawhorse.groovy:4

    def headerClosed() {
        [a: new Auger()].each { String s,
                                Auger c -> c.bore() }
        c.bore()
      //^ d: src/main/groovy/rebound/Sawhorse.groovy:4
    }

    def headerOtherName() {
        [a: new Auger()].each { String s,
                                Auger d ->
            c.bore()
          //^ d: src/main/groovy/rebound/Sawhorse.groovy:4
        }
    }

    def defaultWithComma() {
        def f = { Auger c = make(1, 2) -> c.bore() }
        //                                ^ d: src/main/groovy/rebound/Sawhorse.groovy:67
    }

    def defaultWithCommaClosed() { def f = { Auger c = make(1, 2) -> c }; c.bore() }
    //                                                               ^ d: src/main/groovy/rebound/Sawhorse.groovy:71
    //                                                                    ^ d: src/main/groovy/rebound/Sawhorse.groovy:4

    def commandCall(Auger c) {
        Thread.sleep c
        c.bore()
      //^ d: src/main/groovy/rebound/Sawhorse.groovy:75
    }

    def commandCallCapital(Auger c) {
        Utils.log c
        c.bore()
      //^ d: src/main/groovy/rebound/Sawhorse.groovy:81
    }

    def defCommandCall() {
        def x = new Auger()
        def r = add 1, x
        x.bore()
      //^ d: src/main/groovy/rebound/Sawhorse.groovy:88
    }

    def siblingClosures() {
        foo(1,
            { x ->
                x
              //^ d: src/main/groovy/rebound/Sawhorse.groovy:96
            },
            {
                x
              //^ d: src/main/groovy/rebound/Sawhorse.groovy:5
            })
    }

    def typedInClosure() {
        Auger c = new Auger()
        [1].each { c.bore() }
        //           ^ d: src/main/groovy/rebound/Auger.groovy:4
    }

    def fieldTyped() { c.bore() }
    //                   ^ d: src/main/groovy/rebound/Gimlet.groovy:4

    def bareDefs() { def a, x; x }
    //                         ^ d: src/main/groovy/rebound/Sawhorse.groovy:115

    def threeBareDefs() { def a, b, x; x }
    //                                 ^ d: src/main/groovy/rebound/Sawhorse.groovy:118

    def parenlessCall() { assertEquals c, x; x }
    //                                       ^ d: src/main/groovy/rebound/Sawhorse.groovy:5

    def parenlessDotted() { System.out.println c, x; x }
    //                                               ^ d: src/main/groovy/rebound/Sawhorse.groovy:5

    def outsideTyped() { Thread t = make(); t.bore() }
    //                                        ^ d: picker src/main/groovy/rebound/Auger.groovy:4, src/main/groovy/rebound/Gimlet.groovy:4
}
