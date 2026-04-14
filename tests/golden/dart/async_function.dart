import 'package:myapp/models/user.dart';

// User

Future<User> fetchUser(String id) {
  return await api.fetchUser(id);
}
