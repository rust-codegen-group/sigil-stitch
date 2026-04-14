import 'dart:async';
import 'dart:convert';

// Future

abstract class UserRepository {
  User? findById(String id);

  List<User> findAll();
}

/// In-memory implementation of UserRepository.
class InMemoryUserRepository implements UserRepository {
  final List<User> _users = [];

  @override
  User? findById(String id) {
    return _users.cast<User?>().firstWhere(
      (u) => u?.id == id,
      orElse: () => null,
    );
  }

  @override
  List<User> findAll() {
    return List.unmodifiable(_users);
  }
}

User parseUser(String json) {
  final data = jsonDecode(json);
  return User.fromMap(data);
}
