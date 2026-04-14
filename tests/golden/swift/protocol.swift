/// Generic data repository.
protocol Repository<T> {
    func findById(id: String) -> T?

    func save(entity: T)

    func delete(id: String)
}
