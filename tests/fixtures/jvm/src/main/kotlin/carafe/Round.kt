package carafe

class Round(val f: Flagon) {
    fun checked() {
        if (f is Stein) {
            f.rinse()
            //^ d: src/main/kotlin/carafe/Stein.kt:6
        }
        when (f) {
            is Tankard -> f.pour()
            //              ^ d: src/main/kotlin/carafe/Tankard.kt:4
            else -> {}
        }
        f.pour()
        //^ d: src/main/kotlin/carafe/Flagon.kt:4
    }

    fun guarded(x: Flagon) {
        if (x is Stein) {
            x.rinse()
            //^ d: src/main/kotlin/carafe/Stein.kt:6
        }
        x.rinse()
        //^ d: src/main/kotlin/carafe/Flagon.kt:6
        if (x !is Tankard) return
        x.pour()
        //^ d: src/main/kotlin/carafe/Tankard.kt:4
    }

    fun exits(x: Flagon) {
        if (x is Stein) {
            x.rinse()
            //^ d: src/main/kotlin/carafe/Stein.kt:6
        } else {
            return
        }
        x.swirl()
        //^ d: picker src/main/kotlin/carafe/Flagon.kt:8, src/main/kotlin/carafe/Stein.kt:8, src/main/kotlin/carafe/Tankard.kt:8, src/main/kotlin/ladle/LadleExt.kt:8, src/main/kotlin/press/Brewing.kt:5, src/main/kotlin/press/Carafe.kt:8
    }
}
