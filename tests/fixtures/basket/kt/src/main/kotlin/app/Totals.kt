package app

fun doubled(prices: List<Int>): List<Int> {
    val out = mutableListOf<Int>()
    val seen = ArrayList<Int>()
    for (p in prices) {
        out.add(p * 2)
        //  ^ d: none
        seen.add(p)
        //   ^ d: none
    }
    val text = "a,b"
    return if (text.contains("a")) out else seen
    //              ^ d: none
}
