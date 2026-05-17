public class Utils {
    public static <T extends Comparable> List<T> sortList(List<T> list) {
        Collections.sort(list);
        return list;
    }
}
