interface Repository {
    public function findById(string $id):?User;
    public function save(User $entity): void;
}
