internal enum class Status {
    ACTIVE("active"),
    INACTIVE("inactive");

    internal val value: String

    internal fun getValue(): String {
        return value
    }
}
