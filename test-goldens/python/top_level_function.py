from json import dumps


def serialize(value: object) -> str:
    """Serialize an object to JSON."""
    return dumps(value)
