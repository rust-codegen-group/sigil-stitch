/**
 * Abstract shape.
 */
internal abstract class Shape {
    fun describe(): String {
        return this::class.simpleName ?: "Shape"
    }

    abstract fun area(): Double
}
