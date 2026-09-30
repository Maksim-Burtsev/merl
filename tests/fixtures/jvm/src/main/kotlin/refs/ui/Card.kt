package refs.ui

import refs.model.Topic
import refs.model.asExternalModel

fun card(topics: List<Topic>) {
    topics.map(Topic::asExternalModel)
    //                ^ d: src/main/kotlin/refs/model/Topic.kt:5
    // status: asExternalModel → Topic.asExternalModel (via Topic)
    //         ^ d: src/main/kotlin/refs/model/Topic.kt:3
    // status: Topic: by name, 1 match
    val kind = Topic::class
    //                ^ d: none
}
