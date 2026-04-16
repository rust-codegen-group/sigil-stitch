public class Config {
    public Config(String name, int value) {
        this.name = name;
        this.value = value;
    }

    public Config(String name) {
        this(name, 0);
        // default value constructor
    }
}
