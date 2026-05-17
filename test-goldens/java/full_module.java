import java.util.ArrayList;
import java.util.List;

import javax.annotation.Nullable;

public interface UserRepository {
    @Nullable
    User findById(String id);

    List<User> findAll();
}

/**
 * In-memory implementation of UserRepository.
 */
public class InMemoryUserRepository implements UserRepository {
    private final List<User> users;

    public InMemoryUserRepository() {
        this.users = new ArrayList<>();
    }

    @Override
    @Nullable
    public User findById(String id) {
        return this.users.stream()
            .filter(u -> u.getId().equals(id))
            .findFirst()
            .orElse(null);
    }

    @Override
    public List<User> findAll() {
        return new ArrayList<>(this.users);
    }
}
