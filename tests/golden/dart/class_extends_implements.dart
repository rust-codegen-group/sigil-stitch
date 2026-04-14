import 'package:myapp/auth.dart';
import 'package:myapp/base.dart';
import 'package:myapp/serial.dart';

class AdminService extends BaseService implements Authenticatable, Serializable {
  bool isAdmin() {
    return true;
  }
}
