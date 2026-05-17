public class SortedList<T>
    where T : IComparable
{
    private List<T> items;

    public void Add(T item) {
        items.Add(item);
        items.Sort();
    }
}
