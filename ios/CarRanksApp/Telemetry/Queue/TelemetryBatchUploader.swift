import Foundation

/// Upload abstraction so queue logic can be tested without real networking.
@MainActor
protocol TelemetryBatchUploader: AnyObject {
    func upload(batch: TelemetryBatchRequest) async throws -> TelemetryBatchUploadResponse
}
