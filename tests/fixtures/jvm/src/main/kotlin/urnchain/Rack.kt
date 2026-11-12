package urnchain

import quern.bins.TreeMap

class Rack(private val byName: TreeMap<String, Crock>) {
    fun heat() {
        byName.get("a")!!.boil()
        //                ^ d: !jump
    }
}
