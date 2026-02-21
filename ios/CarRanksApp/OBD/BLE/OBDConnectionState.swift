import Foundation

/// Explicit connection lifecycle so UI rendering and retry logic remain deterministic.
enum OBDConnectionState: Equatable {
    case disconnected
    case scanning
    case connecting(String)
    case reconnecting(name: String, attempt: Int, maxAttempts: Int)
    case connected(String)
    case error(String)

    var isConnected: Bool {
        if case .connected = self {
            return true
        }
        return false
    }

    var statusText: String {
        switch self {
        case .disconnected:
            return "Disconnected"
        case .scanning:
            return "Scanning for adapters"
        case let .connecting(name):
            return "Connecting to \(name)"
        case let .reconnecting(name, attempt, maxAttempts):
            return "Reconnecting to \(name) (\(attempt)/\(maxAttempts))"
        case let .connected(name):
            return "Connected to \(name)"
        case let .error(message):
            return "Error: \(message)"
        }
    }
}
