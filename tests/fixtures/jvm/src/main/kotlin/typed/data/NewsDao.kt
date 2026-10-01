package typed.data

interface NewsDao {
    suspend fun purgeAll(ids: List<String>)
}
