internal class Student(val name: String, val grade: Int) : Person {
    internal override fun toString(): String {
        return "Student($name, grade=$grade)"
    }
}
