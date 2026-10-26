package resources

import app.feature.settings.R.string

fun removedAgain(): String = stringResource(string.bookmark_removed)
//                                                 ^ d: src/main/res/values/strings.xml:2
fun localString(string: Labels): String = string.bookmark_removed
//                                               ^ d: none
