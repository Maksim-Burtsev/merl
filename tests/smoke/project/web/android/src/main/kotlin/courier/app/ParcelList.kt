package courier.app

class ParcelList(private val sync: ParcelSync) {
    fun refresh(codes: List<String>): Int {
        val trimmed = codes.map { code -> code.trim() }
        return sync.send(trimmed)
    }

    fun push(target: Any, codes: List<String>) {
        if (target is ParcelSync) target.send(codes)
    }
}
