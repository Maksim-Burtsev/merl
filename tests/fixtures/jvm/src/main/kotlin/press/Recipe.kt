package press

class Recipe(val grounds: Int, val water: Int) {
    class Builder {
        private var grams = 0

        fun grounds(g: Int): Builder {
            grams = g
            return this
        }

        fun water(w: Int): Builder = this

        fun finish(): Recipe = Recipe(grams, 0)
    }
}
