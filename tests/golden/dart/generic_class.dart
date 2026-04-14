/// A sorted list with bounded type parameter.
class SortedList<T extends Comparable> {
  final List<T> items;

  void add(T item) {
    items.add(item);
    items.sort();
  }
}
