package refs.ui

import refs.model.Topic

private val Topic.testTag get() = "topic:$id"

fun tag(topic: Topic) = topic.testTag
//                            ^ d: src/main/kotlin/refs/ui/Tags.kt:5
// status: testTag → Topic.testTag (via topic: Topic)
