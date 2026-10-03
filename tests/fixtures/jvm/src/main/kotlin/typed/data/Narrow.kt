package typed.data

class Narrow {
    suspend fun cast(dao: Any, ids: List<String>) {
        val news = dao as NewsDao
        news.purgeAll(ids)
        //   ^ d: src/main/kotlin/typed/data/NewsDao.kt:4
        // status: via news: NewsDao
        (dao as TopicDao).purgeAll(ids)
        //                ^ d: src/main/kotlin/typed/data/TopicDao.kt:4
        // status: via TopicDao
        (dao as? TopicDao)?.purgeAll(ids)
        //                  ^ d: src/main/kotlin/typed/data/TopicDao.kt:4
    }

    suspend fun smart(dao: Any, ids: List<String>) {
        if (dao is TopicDao) {
            dao.purgeAll(ids)
            //  ^ d: src/main/kotlin/typed/data/TopicDao.kt:4
            // status: via dao: TopicDao
        } else {
            dao.purgeAll(ids)
            //  ^ d: picker src/main/kotlin/typed/data/NewsDao.kt:4, src/main/kotlin/typed/data/TopicDao.kt:4
        }
        if (dao is NewsDao && ids.isEmpty()) dao.purgeAll(ids)
        //                                       ^ d: src/main/kotlin/typed/data/NewsDao.kt:4
        if (dao is NewsDao) dao.hashCode() else dao.purgeAll(ids)
        //                                          ^ d: picker src/main/kotlin/typed/data/NewsDao.kt:4, src/main/kotlin/typed/data/TopicDao.kt:4
    }

    suspend fun branch(dao: Any, ids: List<String>) {
        when (dao) {
            is NewsDao -> dao.purgeAll(ids)
            //                ^ d: src/main/kotlin/typed/data/NewsDao.kt:4
            is TopicDao -> {
                dao.purgeAll(ids)
                //  ^ d: src/main/kotlin/typed/data/TopicDao.kt:4
            }
            else -> dao.purgeAll(ids)
            //          ^ d: picker src/main/kotlin/typed/data/NewsDao.kt:4, src/main/kotlin/typed/data/TopicDao.kt:4
        }
        when {
            dao is TopicDao -> dao.purgeAll(ids)
            //                     ^ d: src/main/kotlin/typed/data/TopicDao.kt:4
        }
    }

    suspend fun entry(entry: Map.Entry<String, NewsDao>, ids: List<String>) {
        entry.purgeAll(ids)
        //    ^ d: none
    }

    suspend fun safe(news: NewsDao?, ids: List<String>) {
        news?.purgeAll(ids)
        //    ^ d: src/main/kotlin/typed/data/NewsDao.kt:4
        news!!.purgeAll(ids)
        //     ^ d: src/main/kotlin/typed/data/NewsDao.kt:4
    }
}
