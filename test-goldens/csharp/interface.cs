/// Generic data repository.
public interface IRepository<T> {
    T FindById(string id);

    void Save(T entity);
}
