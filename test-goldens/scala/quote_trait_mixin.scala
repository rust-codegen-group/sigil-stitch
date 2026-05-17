trait Serializable {
  def serialize(): String
}
class User(val name: String) extends Entity with Serializable {
  def serialize(): String = name
}
