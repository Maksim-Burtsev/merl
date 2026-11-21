package hearthside

class Hearth {
    Skillet skillet = new Skillet()

    int scorch() { 2 }

    int scoped() {
        skillet.with { scorch() }
        //             ^ d: picker src/main/groovy/hearthside/Hearth.groovy:6, src/main/groovy/hearthside/Skillet.groovy:4
        skillet.tap { scorch() }
        //            ^ d: picker src/main/groovy/hearthside/Hearth.groovy:6, src/main/groovy/hearthside/Skillet.groovy:4
        def c = { scorch() }
        //        ^ d: src/main/groovy/hearthside/Skillet.groovy:4
        c.delegate = skillet
        c.resolveStrategy = Closure.DELEGATE_FIRST
        [1].each { scorch() }
        //         ^ d: src/main/groovy/hearthside/Hearth.groovy:6
        return c()
    }

    int delegated() {
        def c = {
            scorch()
            // ^ d: src/main/groovy/hearthside/Skillet.groovy:4
        }
        c.delegate = skillet
        c.resolveStrategy = Closure.DELEGATE_ONLY
        def k = { scorch() }
        //        ^ d: src/main/groovy/hearthside/Hearth.groovy:6
        k.delegate = skillet
        return c() + k()
    }
}
