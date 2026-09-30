package typed.data

import kotlinx.coroutines.flow.MutableStateFlow

class Repository(
    private val newsDao: NewsDao,
) {
    private val state = MutableStateFlow(0)

    suspend fun clear(ids: List<String>) = newsDao.purgeAll(ids)
    //                                             ^ d: src/main/kotlin/typed/data/NewsDao.kt:4
    // status: purgeAll → NewsDao.purgeAll (via newsDao: NewsDao)

    fun reference() = newsDao::purgeAll
    //                         ^ d: src/main/kotlin/typed/data/NewsDao.kt:4

    fun bump() = state.purgeAll(emptyList())
    //                 ^ d: none

    suspend fun cast(dao: Any, ids: List<String>) {
        if (dao is TopicDao) dao.purgeAll(ids)
        //                       ^ d: picker src/main/kotlin/typed/data/NewsDao.kt:4, src/main/kotlin/typed/data/TopicDao.kt:4
    }

    private fun dao(): NewsDao = newsDao

    suspend fun hop(ids: List<String>) {
        val found = dao()
        found.purgeAll(ids)
        //    ^ d: src/main/kotlin/typed/data/NewsDao.kt:4
    }
}
