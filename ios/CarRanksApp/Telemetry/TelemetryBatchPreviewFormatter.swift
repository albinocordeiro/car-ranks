import Foundation

/// Encodes telemetry batches for on-device debug inspection before upload.
enum TelemetryBatchPreviewFormatter {
    static func prettyPrintedJSON(from request: TelemetryBatchRequest) throws -> String {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(request)
        return String(decoding: data, as: UTF8.self)
    }
}
