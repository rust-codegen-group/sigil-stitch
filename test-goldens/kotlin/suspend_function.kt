import com.example.model.User

suspend fun fetchUser(id: String): User {
    return api.fetchUser(id)
}
