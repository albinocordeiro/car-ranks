import XCTest
@testable import CarRanksApp

final class TelemetryBatchPreviewFormatterTests: XCTestCase {
    func testPrettyPrintedJSONContainsStableContractKeys() throws {
        let request = TelemetryBatchRequest(
            batchID: UUID(uuidString: "6d6401de-02b6-45df-86ab-3e97c95cf80c")!,
            vehicleUID: UUID(uuidString: "1df2e819-c57e-4fd6-9808-65fcb5f613b0")!,
            client: .init(appVersion: "1.0.0", adapterFingerprint: "fingerprint"),
            captureWindow: .init(
                startedAt: "2026-02-17T10:30:00.000Z",
                endedAt: "2026-02-17T10:31:00.000Z",
                sampleIntervalSeconds: 60
            ),
            records: [],
            sessionEvents: [],
            diagnostics: []
        )

        let json = try TelemetryBatchPreviewFormatter.prettyPrintedJSON(from: request)

        XCTAssertTrue(json.contains("\"batch_id\""))
        XCTAssertTrue(json.contains("\"capture_window\""))
        XCTAssertTrue(json.contains("\"schema_version\""))
        XCTAssertTrue(json.contains("\"source\""))
        XCTAssertTrue(json.contains("\n"))
    }
}
