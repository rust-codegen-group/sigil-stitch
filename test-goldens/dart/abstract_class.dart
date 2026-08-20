/// Abstract shape.
abstract class Shape {
  String describe() {
    return runtimeType.toString();
  }

  double area();
}
