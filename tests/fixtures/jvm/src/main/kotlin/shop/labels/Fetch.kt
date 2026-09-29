package shop.labels

import io.ktor.client.HttpClient

class Api {
    fun get(url: String, timeout: Int = 0): String = url
}

// `get` is the library's `HttpClient.get`: the project's namesake is only offered.
fun load(client: HttpClient): String {
    return client.get("x", timeout = 5)
    //                     ^ d: picker src/main/kotlin/shop/labels/Fetch.kt:6
}
