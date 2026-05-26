/**
 * Repository defines data access methods.
 */
interface Repository {
    function findById(string $id): ?User;

    function save(User $entity): void;
}
