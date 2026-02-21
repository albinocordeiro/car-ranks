import Foundation

/// Storage abstraction to keep queued telemetry batches durable across app launches.
protocol TelemetryUploadQueueStore: AnyObject {
    func load() throws -> [TelemetryPendingBatch]
    func save(_ queue: [TelemetryPendingBatch]) throws
}
