package urnchain

class Crock {
    fun boil() {}

    fun size(n: Int): Crock = this

    companion object {
        fun builder(): CrockBuilder = CrockBuilder()
    }
}

class CrockBuilder {
    fun size(n: Int): CrockBuilder = this

    fun build(): Crock = Crock()
}

class Wok {
    fun boil() {}

    fun size(n: Int): Wok = this

    fun build(): Wok = this
}
