package typed.data

interface TopicDao {
    suspend fun purgeAll(ids: List<String>)
}
