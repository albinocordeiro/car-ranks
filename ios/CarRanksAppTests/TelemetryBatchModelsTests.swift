import XCTest
@testable import CarRanksApp

final class TelemetryBatchModelsTests: XCTestCase {
    func testSignalRecordMapsFromOBDRecord() {
        let observedAt = Date(timeIntervalSince1970: 1_700_000_000)
        let sessionID = UUID(uuidString: "b4e88ed2-3dbe-4ac2-9f14-f64f147ac98a")!
        let obdRecord = OBDSignalRecord(
            observedAt: observedAt,
            sessionID: sessionID,
            signalKey: "speed.vehicle",
            valueNumber: 38.2,
            unit: "km/h",
            status: .ok,
            confidence: 0.98,
            sourceSignal: "01_0D",
            rawPayloadRef: "cmd=010D resp=41 0D 26"
        )

        let mapped = TelemetryBatchRequest.SignalRecord.from(obdRecord: obdRecord)

        XCTAssertEqual(mapped.signalKey, "speed.vehicle")
        XCTAssertEqual(mapped.valueNumber, 38.2)
        XCTAssertEqual(mapped.status, "ok")
        XCTAssertEqual(mapped.unit, "km/h")
        XCTAssertEqual(mapped.sourceSignal, "01_0D")
        XCTAssertEqual(mapped.rawPayloadRef, "cmd=010D resp=41 0D 26")
        XCTAssertEqual(mapped.sessionID, sessionID)
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
                    sessionID: UUID(uuidString: "9b2f73ce-6f8d-429f-9b9a-68b49b4e84ff"),
                    signalKey: "speed.vehicle",
                    valueNumber: 48.3,
                    valueString: nil,
                    valueBool: nil,
                    valueJSON: nil,
                    unit: "km/h",
                    status: "ok",
                    confidence: 0.98,
                    sourceSignal: "01_0D",
                    rawPayloadRef: "cmd=010D resp=41 0D 30"
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
        let records = try XCTUnwrap(payload["records"] as? [[String: Any]])
        XCTAssertEqual(
            records.first?["session_id"] as? String,
            "9b2f73ce-6f8d-429f-9b9a-68b49b4e84ff".uppercased()
        )
    }

    func testDiagnosticEventMapsFromDiagnosticSnapshot() {
        let snapshot = OBDDiagnosticSnapshot(
            observedAt: Date(timeIntervalSince1970: 1_700_000_000),
            milOn: true,
            dtcsActive: ["p010a", "U0123", "U0123"]
        )

        let mapped = TelemetryBatchRequest.DiagnosticEvent.from(snapshot: snapshot)

        XCTAssertEqual(mapped.milOn, true)
        XCTAssertEqual(mapped.dtcsActive, ["P010A", "U0123"])
        XCTAssertFalse(mapped.observedAt.isEmpty)
    }
}
