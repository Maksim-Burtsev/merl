package resources

import app.core.ui.R as CoreUiR

val bookmark_gap = 1

fun removed(): String = stringResource(id = R.string.bookmark_removed)
//                                                   ^ d: src/main/res/values/strings.xml:2
// status: bookmark_removed: via src/main/res/values/strings.xml
fun hello(): String = stringResource(R.string.greeting)
//                                            ^ d: picker src/main/res/values/strings.xml:3, src/main/res/values-de/strings.xml:2
fun aliased(): String = stringResource(CoreUiR.string.bookmark_removed)
//                                                    ^ d: src/main/res/values/strings.xml:2
fun tint(): Int = colorResource(R.color.accent)
//                                      ^ d: src/main/res/values/strings.xml:5
fun count(): Int = pluralResource(R.plurals.parcels)
//                                          ^ d: src/main/res/values/strings.xml:6
fun icon(): Int = painterResource(R.drawable.ic_parcel)
//                                           ^ d: src/main/res/drawable/ic_parcel.xml:1
fun title(): Int = R.id.parcel_title
//                      ^ d: src/main/res/layout/parcel_row.xml:3
fun gap(): Int = dimensionResource(R.dimen.bookmark_gap)
//                                         ^ d: none
// status: no definition for bookmark_gap
fun platform(): Int = android.R.string.bookmark_removed
//                                     ^ d: none
fun shadowed(R: Labels): String = R.string.bookmark_removed
//                                         ^ d: none
