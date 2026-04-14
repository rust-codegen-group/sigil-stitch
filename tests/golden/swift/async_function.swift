import MyModule

async func fetchUser(id: String) -> User {
    return try await api.fetchUser(id: id)
}
