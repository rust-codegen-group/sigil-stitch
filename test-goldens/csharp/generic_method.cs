public class Utils {
    public static T Max<T>(T a, T b)
        where T : IComparable<T>
    {
        return a.CompareTo(b) > 0 ? a : b;
    }
}
