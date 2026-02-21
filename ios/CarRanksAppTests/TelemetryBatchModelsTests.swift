import XCTest
@testable import CarRanksApp

final class TelemetryBatchModelsTests: XCTestCase {
    func testSignalRecordMapsFromOBDRecord() {
        let observedAt = Date(timeIntervalSince1970: 1_700_000_000)
        let obdRecord = OBDSignalRecord(
            observedAt: observedAt,
            signalKey: "speed.vehicle",
            valueNumber: 38.2,
            unit: "km/h",
            status: .ok,
            confidence: 0.98,
            sourceSignal: "01_0D"
        )

        let mapped = TelemetryBatchRequest.SignalRecord.from(obdRecord: obdRecord)

        XCTAssertEqual(mapped.signalKey, "speed.vehicle")
        XCTAssertEqual(mapped.valueNumber, 38.2)
        XCTAssertEqual(mapped.status, "ok")
        XCTAssertEqual(mapped.unit, "km/h")
        XCTAssertEqual(mapped.sourceSignal, "01_0D")
        XCTAssertNotNil(mapped.observedAt)
    }

    func testTelemetryBatchEncodingUsesContractFieldNames() throws {
        let request = TelemetryBatchRequest(
            batchID: UUID(uuidString: "6d6401de-02b6-45df-86ab-3e97c95cf80c")!,
            vehicleUID: UUID(uuidString: "1df2e819-c57e-4fd6-9808-65fcb5f613b0")!,
            client: .init(appVersion: "1.0.0", adapterFingerprint: "fingerprint"),
            captureWindow: .init(
                startedAt: "2026-02-17T10:30:00.000Z",
                endedAt: "2026-02-17T10:31:00.000Z",
                sampleIntervalSeconds: 60
            ),
            records: [
                .init(
                    observedAt: "2026-02-17T10:30:05.000Z",
                    signalKey: "speed.vehicle",
                    valueNumber: 48.3,
                    valueString: nil,
                    valueBool: nil,
                    valueJSON: nil,
                    unit: "km/h",
                    status: "ok",
                    confidence: 0.98,
                    sourceSignal: "01_0D"
                ),
            ],
            sessionEvents: [],
            diagnostics: []
        )

        let encoded = try JSONEncoder().encode(request)
        let payload = try XCTUnwrap(JSONSerialization.jsonObject(with: encoded) as? [String: Any])

        XCTAssertEqual(payload["schema_version"] as? String, "0.2")
        XCTAssertEqual(payload["source"] as? String, "OBD")
        XCTAssertNotNil(payload["batch_id"])
        XCTAssertNotNil(payload["vehicle_uid"])
        XCTAssertNotNil(payload["capture_window"])
        XCTAssertNotNil(payload["records"])
    }
}
