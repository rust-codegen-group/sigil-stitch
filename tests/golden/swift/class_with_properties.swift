/// Service for managing users.
public class UserService {
    private var repo: UserRepository
    private let logger: Logger

    public func findUser(id: String) -> User? {
        return repo.find(by: id)
    }
}
