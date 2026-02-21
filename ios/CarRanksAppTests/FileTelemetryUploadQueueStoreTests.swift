import XCTest
@testable import CarRanksApp

final class FileTelemetryUploadQueueStoreTests: XCTestCase {
    func testSaveThenLoadRoundTrip() throws {
        let temporaryDirectory = FileManager.default.temporaryDirectory
            .appendingPathComponent("car-ranks-queue-tests-\(UUID().uuidString)", isDirectory: true)
        let queueFileURL = temporaryDirectory.appendingPathComponent("queue.json")
        defer {
            try? FileManager.default.removeItem(at: temporaryDirectory)
        }

        let store = FileTelemetryUploadQueueStore(fileURL: queueFileURL)
        let fixedDate = Date(timeIntervalSince1970: 1_700_000_000)
        let queued = [
            TelemetryPendingBatch(
                request: .fileStoreSample(batchID: UUID(uuidString: "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE")!),
                captureWindowEndedAt: fixedDate,
                enqueuedAt: fixedDate
            ),
        ]

        try store.save(queued)
        let loaded = try store.load()

        XCTAssertEqual(loaded, queued)
    }
}

private extension TelemetryBatchRequest {
    static func fileStoreSample(batchID: UUID = UUID()) -> Self {
        TelemetryBatchRequest(
            batchID: batchID,
            vehicleUID: UUID(uuidString: "11111111-2222-3333-4444-555555555555")!,
            client: .init(
                appVersion: "1.0.0",
                adapterFingerprint: "test-adapter"
            ),
            captureWindow: .init(
                startedAt: "2026-02-21T10:00:00Z",
                endedAt: "2026-02-21T10:05:00Z",
                sampleIntervalSeconds: 5
            ),
            records: [],
            sessionEvents: [],
            diagnostics: []
        )
    }
}
