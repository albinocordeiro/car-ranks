import Foundation

/// Retry policy focused on transient transport/server failures.
struct TelemetryUploadRetryPolicy: Equatable {
    let maxAttempts: Int
    let initialDelaySeconds: TimeInterval
    let maxDelaySeconds: TimeInterval
    let backoffMultiplier: Double

    static let standard = TelemetryUploadRetryPolicy(
        maxAttempts: 6,
        initialDelaySeconds: 5,
        maxDelaySeconds: 300,
        backoffMultiplier: 2
    )

    func shouldRetry(error: BackendError, retryCount: Int) -> Bool {
        guard retryCount < maxAttempts else {
            return false
        }

        switch error {
        case .transport, .invalidResponse:
            return true
        case let .server(statusCode, _):
            return statusCode == 408 || statusCode == 429 || (500 ... 599).contains(statusCode)
        case .invalidURL, .decode:
            return false
        }
    }

    func delaySeconds(forRetryCount retryCount: Int) -> TimeInterval {
        guard retryCount > 0 else {
            return 0
        }
        let exponent = Double(max(0, retryCount - 1))
        let rawDelay = initialDelaySeconds * pow(backoffMultiplier, exponent)
        return min(maxDelaySeconds, rawDelay)
    }
}
