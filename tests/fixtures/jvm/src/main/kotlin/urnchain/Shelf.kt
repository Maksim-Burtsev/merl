package urnchain

class Shelf(private val byName: Map<String, Crock>) {
    private val rows: Map<String, List<Crock>> = emptyMap()
    private val woks: MutableMap<String, Wok?> = mutableMapOf()

    fun heat() {
        byName["a"]!!.boil()
        //            ^ d: src/main/kotlin/urnchain/Crock.kt:4
        byName["a"]?.boil()
        //           ^ d: src/main/kotlin/urnchain/Crock.kt:4
        byName.getValue("a").boil()
        //                   ^ d: src/main/kotlin/urnchain/Crock.kt:4
        byName.get("a")!!.boil()
        //                ^ d: src/main/kotlin/urnchain/Crock.kt:4
        woks["a"]?.boil()
        //         ^ d: src/main/kotlin/urnchain/Crock.kt:20
        rows["a"]!![0].boil()
        //             ^ d: !jump
    }
}
