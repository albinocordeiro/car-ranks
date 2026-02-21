import Foundation

/// Tunable reconnect behavior for transient BLE drops during active capture sessions.
struct OBDReconnectPolicy: Equatable {
    let maxAttempts: Int
    let initialDelaySeconds: TimeInterval
    let maxDelaySeconds: TimeInterval
    let backoffMultiplier: Double

    static let standard = OBDReconnectPolicy(
        maxAttempts: 4,
        initialDelaySeconds: 1,
        maxDelaySeconds: 8,
        backoffMultiplier: 2
    )

    func shouldRetry(attempt: Int) -> Bool {
        attempt > 0 && attempt <= maxAttempts
    }

    func delaySeconds(forAttempt attempt: Int) -> TimeInterval {
        guard attempt > 0 else { return 0 }
        let exponent = Double(max(0, attempt - 1))
        let delayed = initialDelaySeconds * pow(backoffMultiplier, exponent)
        return min(maxDelaySeconds, delayed)
    }
}
