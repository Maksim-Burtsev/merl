package refs.model

data class Topic(val id: String)

fun Topic.asExternalModel() = this

fun String.asExternalModel() = this
