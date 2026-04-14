/// Abstract shape base class.
class Shape {
    func describe() -> String {
        return String(describing: type(of: self))
    }

    func area() -> Double {
        fatalError("Subclasses must override")
    }
}
