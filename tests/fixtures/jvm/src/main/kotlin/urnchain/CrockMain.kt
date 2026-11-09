package urnchain

class Pot {
    fun boil() {}
}

fun brew() {
    Crock.builder().size(2).build().boil()
    //    ^ d: src/main/kotlin/urnchain/Crock.kt:9
    //              ^ d: src/main/kotlin/urnchain/Crock.kt:14
    //                      ^ d: src/main/kotlin/urnchain/Crock.kt:16
    //                              ^ d: src/main/kotlin/urnchain/Crock.kt:4
    Crock.builder()
        .size(2)
        //^ d: src/main/kotlin/urnchain/Crock.kt:14
        .build()
        //^ d: src/main/kotlin/urnchain/Crock.kt:16
        .boil()
        //^ d: src/main/kotlin/urnchain/Crock.kt:4
    Mystery.builder().size(2).boil()
    //                ^ d: !jump
    //                        ^ d: !jump
}
