package alembic

import alembic.lab.Still as Vat

fun run(): Int {
    val a = Vat()
    //      ^ d: src/main/kotlin/alembic/lab/Still.kt:3
    a.condense()
    //^ d: src/main/kotlin/alembic/lab/Still.kt:4
    return Vat().condense()
    //           ^ d: src/main/kotlin/alembic/lab/Still.kt:4
}
