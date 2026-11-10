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
