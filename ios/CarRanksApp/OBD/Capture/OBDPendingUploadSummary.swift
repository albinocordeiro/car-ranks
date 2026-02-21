import Foundation

/// Compact counter model that explains what will be sent on the next telemetry upload.
struct OBDPendingUploadSummary: Equatable {
    let signalRecordCount: Int
    let diagnosticEventCount: Int
    let sessionEventCount: Int

    static let empty = OBDPendingUploadSummary(
        signalRecordCount: 0,
        diagnosticEventCount: 0,
        sessionEventCount: 0
    )

    static func from(batch: TelemetryBatchRequest) -> OBDPendingUploadSummary {
        OBDPendingUploadSummary(
            signalRecordCount: batch.records.count,
            diagnosticEventCount: batch.diagnostics.count,
            sessionEventCount: batch.sessionEvents.count
        )
    }

    var inlineDescription: String {
        [
            Self.countText(signalRecordCount, singular: "signal"),
            Self.countText(diagnosticEventCount, singular: "diagnostic"),
            Self.countText(sessionEventCount, singular: "session event"),
        ]
        .joined(separator: ", ")
    }

    private static func countText(_ count: Int, singular: String) -> String {
        if count == 1 {
            return "1 \(singular)"
        }
        return "\(count) \(singular)s"
    }
}
