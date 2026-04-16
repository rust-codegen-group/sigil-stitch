/// Result of a network request.
public enum NetworkResult {
    case success(Data)
    case failure(Error, Int)
    case loading
}
