import Foundation

protocol DataFetcher {
    func fetchData(from: URL) async -> Data
}

/// API response model.
struct Response {
    let statusCode: Int
    let body: Data
}

/// Network-based data fetcher.
class NetworkFetcher: DataFetcher {
    private let session: URLSession

    func fetchData(from: URL) async -> Data {
        let (data, _) = try await session.data(from: from)
        return data
    }
}

func makeURL(urlString: String) -> URL {
    return URL(string: urlString)!
}
