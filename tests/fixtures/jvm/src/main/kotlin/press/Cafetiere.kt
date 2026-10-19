package press

import press.parts.*
import press.plunge as push

/*
 * class Carafe {
 *     fun slosh(): Int = 0
 * }
 */
class Cafetiere(private val carafe: Carafe) : Carafe(), Lever {
    private var plunger: Plunger = Plunger(1, Mesh())

    override fun slosh(): Int = super.slosh() + raise() + swirl() + brim
    //                                ^ d: src/main/kotlin/press/Carafe.kt:6
    //                                          ^ d: src/main/kotlin/press/Lever.kt:4
    //                                                    ^ d: src/main/kotlin/press/Carafe.kt:8
    //                                                              ^ d: src/main/kotlin/press/Carafe.kt:4

    override fun lower(): Int = 0

    fun stdlib(): Int {
        val note = " fun slosh(): Int = 0 "
        val mesh = Mesh()
        return note.trimIndent().length + mesh.trimIndent() + note.steepness()
        //          ^ d: none
        //                       ^ d: none
        //                                     ^ d: src/main/kotlin/press/Mesh.kt:6
        //                                                         ^ d: src/main/kotlin/press/Brewing.kt:3
    }

    fun chain(): Int {
        val built = Recipe.Builder()
            .grounds(2)
        //   ^ d: src/main/kotlin/press/Recipe.kt:7
            .water(3)
        //   ^ d: picker src/main/kotlin/press/Recipe.kt:3, src/main/kotlin/press/Recipe.kt:12; want src/main/kotlin/press/Recipe.kt:12 (#1133)
            .finish()
        //   ^ d: src/main/kotlin/press/Recipe.kt:14
        return built.grounds + Recipe.Builder().grounds(1).finish().water
        //           ^ d: picker src/main/kotlin/press/Recipe.kt:3, src/main/kotlin/press/Recipe.kt:7; want src/main/kotlin/press/Recipe.kt:3 (#1133)
        //                                      ^ d: src/main/kotlin/press/Recipe.kt:7
        //                                                          ^ d: picker src/main/kotlin/press/Recipe.kt:3, src/main/kotlin/press/Recipe.kt:12; want src/main/kotlin/press/Recipe.kt:3 (#1133)
    }

    fun scoped(): Int {
        val a = carafe.let { it.swirl() }
        //                      ^ d: picker src/main/kotlin/press/Brewing.kt:5, src/main/kotlin/press/Carafe.kt:8; want src/main/kotlin/press/Carafe.kt:8 (#1132)
        val b = with(carafe) { slosh() + brim }
        //                     ^ d: src/main/kotlin/press/Cafetiere.kt:14; want src/main/kotlin/press/Carafe.kt:6 (#1132)
        val c = carafe.apply { slosh() }.brim
        //                               ^ d: picker src/main/kotlin/press/Cafetiere.kt:95, src/main/kotlin/press/Carafe.kt:4; want src/main/kotlin/press/Carafe.kt:4 (#1132)
        val d = plunger.also { it.stroke }.mesh.sieve()
        //                        ^ d: src/main/kotlin/press/Plunger.kt:3
        //                                      ^ d: picker src/main/kotlin/press/Mesh.kt:4, src/main/kotlin/press/parts/Mesh.kt:4; want src/main/kotlin/press/Mesh.kt:4 (#1132)
        return a + b + c + d
    }

    fun shadow(plungers: List<Plunger>): Int {
        val plunge = plunge(2)
        //           ^ d: src/main/kotlin/press/Brewing.kt:7
        var plunger = plunger
        //            ^ d: src/main/kotlin/press/Cafetiere.kt:12
        plunger = plunger.copy(stroke = plunge)
        //        ^ d: src/main/kotlin/press/Cafetiere.kt:62
        //                              ^ d: src/main/kotlin/press/Cafetiere.kt:60
        var total = 0
        for (plunger in plungers) total += plunger.stroke
        //                                 ^ d: src/main/kotlin/press/Cafetiere.kt:68
        for ((stroke, mesh) in plungers) total += stroke + mesh.sieve()
        //                                        ^ d: src/main/kotlin/press/Cafetiere.kt:70
        //                                                      ^ d: picker src/main/kotlin/press/Mesh.kt:4, src/main/kotlin/press/parts/Mesh.kt:4; want src/main/kotlin/press/Mesh.kt:4 (#1133)
        val (stroke, mesh) = plunger
        return total + stroke + mesh.sieve() + push(3) + Gasket().seal()
        //             ^ d: src/main/kotlin/press/Cafetiere.kt:73
        //                           ^ d: picker src/main/kotlin/press/Mesh.kt:4, src/main/kotlin/press/parts/Mesh.kt:4; want src/main/kotlin/press/Mesh.kt:4 (#1133)
        //                                     ^ d: src/main/kotlin/press/Brewing.kt:7
        //                                                        ^ d: src/main/kotlin/press/parts/Gasket.kt:4
    }

    fun nested(): Int {
        fun swirl(): Int = 9
        val lever = object : Lever {
            override fun lower(): Int = brim + swirl()
            //                          ^ d: src/main/kotlin/press/Carafe.kt:4
            //                                 ^ d: src/main/kotlin/press/Cafetiere.kt:82
        }
        return swirl() + lever.raise() + Collar().poise()
        //     ^ d: src/main/kotlin/press/Cafetiere.kt:82
        //                     ^ d: src/main/kotlin/press/Lever.kt:4
        //                                        ^ d: src/main/kotlin/press/Cafetiere.kt:97
    }

    inner class Collar {
        val brim = 5

        fun poise(): Int = brim + this@Cafetiere.brim + raise()
        //                 ^ d: src/main/kotlin/press/Cafetiere.kt:95
        //                                       ^ d: src/main/kotlin/press/Carafe.kt:4
        //                                              ^ d: src/main/kotlin/press/Lever.kt:4
    }
}
