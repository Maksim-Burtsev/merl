package kwcafe

import kwcafe.far.Kwutil.kwpourer

fun shadowed(): Int {
    return kwpourer(cups = 1)
//                  ^ d: src/main/kotlin/kwcafe/far/Kwutil.kt:4
}
