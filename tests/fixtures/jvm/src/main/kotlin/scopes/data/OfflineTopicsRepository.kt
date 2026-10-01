package scopes.data

class OfflineTopicsRepository @Inject constructor(
    @Named("seed") private val seed: String,
) : TopicsRepository {
    override fun getTopics(): List<String> = listOf(seed, FALLBACK)
    //                                                    ^ d: src/main/kotlin/scopes/data/OfflineTopicsRepository.kt:15
    // status: FALLBACK → OfflineTopicsRepository.FALLBACK

    fun first(): String = getTopics().first()
    //                    ^ d: src/main/kotlin/scopes/data/OfflineTopicsRepository.kt:6
    // status: getTopics → OfflineTopicsRepository.getTopics

    companion object {
        const val FALLBACK = "offline"
    }
}

fun offline(): String = OfflineTopicsRepository.FALLBACK
//                                               ^ d: src/main/kotlin/scopes/data/OfflineTopicsRepository.kt:15
