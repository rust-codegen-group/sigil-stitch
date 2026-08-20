internal enum class Status(val value: String) {
    ACTIVE("active"),
    INACTIVE("inactive");

    fun getValue(): String {
        return value
    }
}
