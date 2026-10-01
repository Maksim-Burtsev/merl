package shop.labels

import io.ktor.client.HttpClient

class Api {
    fun get(url: String, timeout: Int = 0): String = url
}

// `get` is the library's `HttpClient.get`, whose parameters have no source (#391).
fun load(client: HttpClient): String {
    return client.get("x", timeout = 5)
    //                     ^ d: none
}
