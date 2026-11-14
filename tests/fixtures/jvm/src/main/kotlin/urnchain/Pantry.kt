package urnchain

class Pantry<Crock>(private val byName: Map<String, Crock>) {
    fun heat() {
        byName["a"]!!.boil()
        //            ^ d: !jump
    }
}
