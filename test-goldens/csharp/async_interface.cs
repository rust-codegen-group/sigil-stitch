public interface IUserService {
    Task<User> GetUserAsync(string id);

    Task SaveUserAsync(User user);
}
