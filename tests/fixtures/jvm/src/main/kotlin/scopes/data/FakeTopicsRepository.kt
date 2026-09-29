package scopes.data

class FakeTopicsRepository(private val seed: String) : TopicsRepository {
    override fun getTopics(): List<String> = listOf("a")

    fun getTopic(id: String): String = getTopics().first { it == id }
    //                                 ^ d: src/main/kotlin/scopes/data/FakeTopicsRepository.kt:4
    // status: getTopics → FakeTopicsRepository.getTopics (via FakeTopicsRepository)
    //                                                           ^ d: src/main/kotlin/scopes/data/FakeTopicsRepository.kt:6
    //                                                     ^ d: none

    fun seeded(): String {
        val (first, second) = seed to DEFAULT
        //                    ^ d: src/main/kotlin/scopes/data/FakeTopicsRepository.kt:3
        //                             ^ d: src/main/kotlin/scopes/data/FakeTopicsRepository.kt:22
        return listOf(first, second).joinToString { part -> part + second }
        //            ^ d: src/main/kotlin/scopes/data/FakeTopicsRepository.kt:13
        //                                                  ^ d: src/main/kotlin/scopes/data/FakeTopicsRepository.kt:16
    }

    companion object {
        const val DEFAULT = "x"
    }
}
