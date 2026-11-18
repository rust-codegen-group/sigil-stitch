class UserService:
    """Service for managing users."""

    _repo: UserRepository

    def get_user(self, user_id: str) -> User:
        return self._repo.find(user_id)

    def save_user(self, user: User) -> None:
        self._repo.save(user)
