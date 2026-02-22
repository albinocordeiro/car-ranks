import XCTest
@testable import CarRanksApp

final class OBDPendingUploadSummaryTests: XCTestCase {
    func testFromBatchUsesContractCounts() {
        let batch = TelemetryBatchRequest(
            batchID: UUID(),
            vehicleUID: UUID(),
            client: .init(appVersion: "1.0.0", adapterFingerprint: "fingerprint"),
            captureWindow: .init(
                startedAt: "2026-02-21T00:00:00.000Z",
                endedAt: "2026-02-21T00:01:00.000Z",
                sampleIntervalSeconds: 5
            ),
            records: [
                .init(
                    observedAt: "2026-02-21T00:00:05.000Z",
                    sessionID: nil,
                    signalKey: "speed.vehicle",
                    valueNumber: 54,
                    valueString: nil,
                    valueBool: nil,
                    valueJSON: nil,
                    unit: "km/h",
                    status: "ok",
                    confidence: 0.99,
                    sourceSignal: "01_0D",
                    rawPayloadRef: nil
                ),
                .init(
                    observedAt: "2026-02-21T00:00:10.000Z",
                    sessionID: nil,
                    signalKey: "soc.display",
                    valueNumber: 72,
                    valueString: nil,
                    valueBool: nil,
                    valueJSON: nil,
                    unit: "%",
                    status: "ok",
                    confidence: 0.99,
                    sourceSignal: "01_5B",
                    rawPayloadRef: nil
                ),
            ],
            sessionEvents: [
                .init(eventType: "drive_session_start", observedAt: "2026-02-21T00:00:00.000Z", sessionID: UUID()),
            ],
            diagnostics: [
                .init(observedAt: "2026-02-21T00:00:30.000Z", milOn: false, dtcsActive: []),
            ]
        )

        let summary = OBDPendingUploadSummary.from(batch: batch)

        XCTAssertEqual(summary.signalRecordCount, 2)
        XCTAssertEqual(summary.diagnosticEventCount, 1)
        XCTAssertEqual(summary.sessionEventCount, 1)
    }

    func testInlineDescriptionFormatsSingularAndPlural() {
        let summary = OBDPendingUploadSummary(
            signalRecordCount: 2,
            diagnosticEventCount: 1,
            sessionEventCount: 0
        )

        XCTAssertEqual(summary.inlineDescription, "2 signals, 1 diagnostic, 0 session events")
    }
}
