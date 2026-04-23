import 'dart:convert';

import 'package:http/http.dart';

final client = Client();
final data = jsonDecode(response.body);
