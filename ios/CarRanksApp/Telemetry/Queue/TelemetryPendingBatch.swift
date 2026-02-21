import Foundation

/// Persisted queue item that survives app restarts until upload succeeds or becomes non-retryable.
struct TelemetryPendingBatch: Codable, Equatable, Identifiable {
    let id: UUID
    let request: TelemetryBatchRequest
    let enqueuedAt: Date
    let captureWindowEndedAt: Date
    var retryCount: Int
    var nextRetryAt: Date?
    var lastErrorMessage: String?

    init(
        request: TelemetryBatchRequest,
        captureWindowEndedAt: Date,
        enqueuedAt: Date = Date()
    ) {
        id = request.batchID
        self.request = request
        self.enqueuedAt = enqueuedAt
        self.captureWindowEndedAt = captureWindowEndedAt
        retryCount = 0
        nextRetryAt = nil
        lastErrorMessage = nil
    }
}
