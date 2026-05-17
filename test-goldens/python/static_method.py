class User:
    @classmethod
    def from_dict(cls, name: str, age: int) -> User:
        return cls(name, age)
