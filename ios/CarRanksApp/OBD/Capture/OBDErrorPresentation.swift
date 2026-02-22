import Foundation

/// Adapter-facing error formatter so OBD failures are actionable during field testing.
enum OBDErrorPresentation {
    static func message(from error: Error) -> String {
        guard let backendError = error as? BackendError else {
            return error.localizedDescription
        }

        switch backendError {
        case let .transport(message):
            return message
        case let .server(_, message):
            return message
        case let .decode(message):
            return "Failed to decode adapter response: \(message)"
        case .invalidURL, .invalidResponse:
            return backendError.displayMessage
        }
    }
}
