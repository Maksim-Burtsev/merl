package rebound

class Brace {
    def x = 1

    def loop(es) { for (x in es) { }; x }
    //                                ^ d: src/main/groovy/rebound/Brace.groovy:4
}
