package refs.ui

import refs.model.Topic
import refs.model.asExternalModel

fun card(topics: List<Topic>) {
    topics.map(Topic::asExternalModel)
    //                ^ d: src/main/kotlin/refs/model/Topic.kt:5
    // status: asExternalModel → Topic.asExternalModel (via import src/main/kotlin/refs/model/Topic.kt)
    //         ^ d: src/main/kotlin/refs/model/Topic.kt:3
    // status: Topic: via import src/main/kotlin/refs/model/Topic.kt
    val kind = Topic::class
    //                ^ d: none
}
