package scopes

data class Change(val id: String, val version: Int)

private fun <T> List<T>.mapToChangeList(
    idGetter: (T) -> String,
) = mapIndexed { index, item ->
    Change(id = idGetter(item), version = index)
    //          ^ d: src/main/kotlin/scopes/Change.kt:6
    // status: idGetter → mapToChangeList.idGetter (local)
    //                   ^ d: src/main/kotlin/scopes/Change.kt:7
    //                                    ^ d: src/main/kotlin/scopes/Change.kt:7
}
