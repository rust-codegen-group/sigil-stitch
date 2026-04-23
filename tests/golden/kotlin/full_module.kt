import kotlin.collections.ArrayList
import kotlin.collections.List
import kotlin.collections.MutableList

internal interface UserRepository {
    internal fun findById(id: String): User?

    internal fun findAll(): List<User>
}

/**
 * In-memory implementation of UserRepository.
 */
internal class InMemoryUserRepository : UserRepository {
    private val users: MutableList<User>

    internal override fun findById(id: String): User? {
        return users.firstOrNull { it.id == id }
    }

    internal override fun findAll(): List<User> {
        return ArrayList<User>(users)
    }
}
