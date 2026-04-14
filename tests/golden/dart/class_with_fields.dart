/// Service for managing users.
class UserService {
  UserRepository repo;
  final Logger logger;

  UserService(UserRepository repo, Logger logger) {
    this.repo = repo;
    this.logger = logger;
  }

  User? findUser(String id) {
    return repo.findById(id);
  }
}
