import MyModule

func fetchUser(id: String) async -> User {
    return try await api.fetchUser(id: id)
}
