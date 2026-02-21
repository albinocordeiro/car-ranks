import XCTest
@testable import CarRanksApp

@MainActor
final class TelemetryUploadQueueCoordinatorTests: XCTestCase {
    func testManualTriggerUploadsQueuedBatch() async {
        let uploader = FakeTelemetryBatchUploader(
            results: [.success(.accepted(batchID: UUID(uuidString: "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE")!))]
        )
        let store = InMemoryTelemetryUploadQueueStore()
        let coordinator = TelemetryUploadQueueCoordinator(
            uploader: uploader,
            store: store,
            autoUploadEnabled: false,
            enableNetworkMonitoring: false
        )
        let batch = TelemetryBatchRequest.sample(batchID: UUID(uuidString: "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE")!)

        coordinator.enqueue(
            batch: batch,
            captureWindowEndedAt: Date(timeIntervalSince1970: 1_700_000_000)
        )
        coordinator.triggerUpload()
        await waitUntilSettled(coordinator)

        XCTAssertEqual(coordinator.pendingBatchCount, 0)
        XCTAssertEqual(uploader.uploadedBatchIDs, [batch.batchID])
        XCTAssertTrue(store.savedQueue.isEmpty)
    }

    func testTransientFailureKeepsBatchAndSchedulesRetry() async {
        let uploader = FakeTelemetryBatchUploader(
            results: [.failure(.server(statusCode: 503, message: "service unavailable"))]
        )
        let store = InMemoryTelemetryUploadQueueStore()
        let policy = TelemetryUploadRetryPolicy(
            maxAttempts: 3,
            initialDelaySeconds: 60,
            maxDelaySeconds: 300,
            backoffMultiplier: 2
        )
        let coordinator = TelemetryUploadQueueCoordinator(
            uploader: uploader,
            store: store,
            retryPolicy: policy,
            autoUploadEnabled: false,
            enableNetworkMonitoring: false
        )

        coordinator.enqueue(
            batch: .sample(),
            captureWindowEndedAt: Date(timeIntervalSince1970: 1_700_000_000)
        )
        coordinator.triggerUpload()
        await waitUntilSettled(coordinator)

        XCTAssertEqual(coordinator.pendingBatchCount, 1)
        XCTAssertEqual(store.savedQueue.first?.retryCount, 1)
        XCTAssertNotNil(store.savedQueue.first?.nextRetryAt)
    }

    func testNonRetryableFailureDropsBatch() async {
        let uploader = FakeTelemetryBatchUploader(
            results: [.failure(.server(statusCode: 401, message: "unauthorized"))]
        )
        let store = InMemoryTelemetryUploadQueueStore()
        let coordinator = TelemetryUploadQueueCoordinator(
            uploader: uploader,
            store: store,
            autoUploadEnabled: false,
            enableNetworkMonitoring: false
        )

        coordinator.enqueue(
            batch: .sample(),
            captureWindowEndedAt: Date(timeIntervalSince1970: 1_700_000_000)
        )
        coordinator.triggerUpload()
        await waitUntilSettled(coordinator)

        XCTAssertEqual(coordinator.pendingBatchCount, 0)
        XCTAssertTrue(store.savedQueue.isEmpty)
        XCTAssertTrue(coordinator.lastUploadMessage?.hasPrefix("Dropped queued telemetry batch") == true)
    }

    private func waitUntilSettled(_ coordinator: TelemetryUploadQueueCoordinator) async {
        for _ in 0 ..< 100 {
            if !coordinator.isUploading {
                // Allow one extra cycle so store writes complete.
                await Task.yield()
                return
            }
            try? await Task.sleep(nanoseconds: 10_000_000)
        }
    }
}

@MainActor
private final class FakeTelemetryBatchUploader: TelemetryBatchUploader {
    private var results: [Result<TelemetryBatchUploadResponse, BackendError>]
    private(set) var uploadedBatchIDs: [UUID] = []

    init(results: [Result<TelemetryBatchUploadResponse, BackendError>]) {
        self.results = results
    }

    func upload(batch: TelemetryBatchRequest) async throws -> TelemetryBatchUploadResponse {
        uploadedBatchIDs.append(batch.batchID)
        if !results.isEmpty {
            switch results.removeFirst() {
            case let .success(response):
                return response
            case let .failure(error):
                throw error
            }
        }
        return .accepted(batchID: batch.batchID)
    }
}

private final class InMemoryTelemetryUploadQueueStore: TelemetryUploadQueueStore {
    private(set) var savedQueue: [TelemetryPendingBatch] = []

    func load() throws -> [TelemetryPendingBatch] {
        savedQueue
    }

    func save(_ queue: [TelemetryPendingBatch]) throws {
        savedQueue = queue
    }
}

private extension TelemetryBatchRequest {
    static func sample(batchID: UUID = UUID()) -> Self {
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

private extension TelemetryBatchUploadResponse {
    static func accepted(batchID: UUID) -> Self {
        TelemetryBatchUploadResponse(
            accepted: true,
            batchID: batchID,
            ingestID: UUID(),
            duplicate: false,
            recordsReceived: 0,
            recordsAccepted: 0,
            recordsRejected: 0,
            errors: [],
            nextUploadAfterSeconds: 0
        )
    }
}
