import Foundation

enum BackendError: Error, Equatable {
    case invalidURL
    case invalidResponse
    case server(statusCode: Int, message: String)
    case decode(String)
    case transport(String)

    var displayMessage: String {
        switch self {
        case .invalidURL:
            return "Invalid API URL configuration."
        case .invalidResponse:
            return "Unexpected API response."
        case let .server(_, message):
            return message
        case let .decode(message):
            return "Failed to decode response: \(message)"
        case let .transport(message):
            return "Network request failed: \(message)"
        }
    }
}
