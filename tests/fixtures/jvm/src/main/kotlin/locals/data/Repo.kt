package locals.data

class Repo @Inject constructor(
    private val api: String,
    val cacheSize: Int,
) : Store() {
    val pending = 1
}

open class Store
