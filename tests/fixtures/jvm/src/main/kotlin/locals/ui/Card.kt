package locals.ui

import androidx.compose.foundation.layout.height
import locals.data.Repo
import locals.data.Vm

fun card(vm: Vm) {
    Spacer(modifier = Modifier.height(14.dp))
    //                         ^ d: none
    // status: no definition for height
    vm.followTopic(followedTopicId = "a", false)
    //             ^ d: none
    // status: no definition for followedTopicId
}

fun size(repo: Repo): Int = repo.cacheSize + repo.pending
//                               ^ d: src/main/kotlin/locals/data/Repo.kt:5
//                                                ^ d: src/main/kotlin/locals/data/Repo.kt:7
