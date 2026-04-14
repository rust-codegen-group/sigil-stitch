/// Abstract shape.
abstract class Shape {
  String describe() {
    return runtimeType.toString();
  }

  abstract double area();
}
