/**
 * Generic data repository.
 */
internal interface Repository<T> {
    fun findById(id: String): T?

    fun save(entity: T)

    fun delete(id: String)
}
