internal enum class Status(val value: String) {
    ACTIVE("active"),
    INACTIVE("inactive");

    internal fun getValue(): String {
        return value
    }
}
