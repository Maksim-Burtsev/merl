package imports.ui

import androidx.compose.ui.res.pluralText
import imports.model.Badge

fun badgeCard(badge: Badge) = badge.id + pluralText(id = 1)
//                   ^ d: src/main/kotlin/imports/model/Badge.kt:3
// status: Badge: via import src/main/kotlin/imports/model/Badge.kt
//                                       ^ d: none
