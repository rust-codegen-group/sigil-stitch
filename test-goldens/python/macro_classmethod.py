@classmethod
def from_dict(cls, data: dict) -> "User":
    return cls(data["name"], data["age"])
