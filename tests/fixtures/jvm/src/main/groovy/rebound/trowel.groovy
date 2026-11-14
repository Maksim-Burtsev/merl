trowel = new Auger()
def spade = new Gimlet()

println trowel
//      ^ d: src/main/groovy/rebound/trowel.groovy:1
println spade
//      ^ d: src/main/groovy/rebound/trowel.groovy:2

def dig() { trowel }
//          ^ d: src/main/groovy/rebound/trowel.groovy:1

def shade(trowel) { trowel }
//                  ^ d: src/main/groovy/rebound/trowel.groovy:12

spigot = 'main'
node {
    build job: 'deploy', parameters: [string(name: 'B', value: spigot)]
    //                                                         ^ d: src/main/groovy/rebound/trowel.groovy:15
    checkout scm: [spigot: 'x']
    //             ^ d: none
}
nozzle = 0
def h = new Ferrule(nozzle: 2)
//                  ^ d: src/main/groovy/rebound/Ferrule.groovy:4
println nozzle
//      ^ d: src/main/groovy/rebound/trowel.groovy:22
class Socket {
    def grip() { nozzle }
    //           ^ d: src/main/groovy/rebound/Ferrule.groovy:4
}
