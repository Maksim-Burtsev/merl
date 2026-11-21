package urnchain

class Pantry<Crock>(private val byName: Map<String, Crock>) {
    fun heat() {
        byName["a"]!!.boil()
        //            ^ d: !jump
    }

    fun pour(kettles: List<Crock>) {
        kettles[0].boil()
        //         ^ d: !jump
    }
}
