@dataclass
class Config:
    """Application configuration."""

    name: str
    port: int
    debug: bool = False
