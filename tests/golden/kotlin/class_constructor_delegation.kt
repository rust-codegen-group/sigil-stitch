internal class Person(val name: String, val age: Int) {
    internal constructor(name: String) : this(name, 0) {
        println("Created with default age")
    }
}
