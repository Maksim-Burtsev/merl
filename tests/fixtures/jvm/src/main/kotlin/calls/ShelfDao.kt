package calls

interface ShelfDao {
    @Query(
        value = """
        SELECT * FROM shelves
        WHERE sku = :sku
    """,
    )
    fun shelf(sku: String): Shelf
}
