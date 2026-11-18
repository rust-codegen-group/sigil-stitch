from typing import Protocol


class Repository(Protocol):
    """Repository defines data access methods."""

    def find_by_id(self, id: str) -> Entity: ...

    def save(self, entity: Entity) -> None: ...
