/**
 * Abstract shape.
 */
abstract class Shape {
  def describe(): String = {
    getClass.getSimpleName
  }

  def area(): Double
}
