import XCTest
@testable import CarRanksApp

@MainActor
final class OBDCaptureViewModelTests: XCTestCase {
    private let replayFixtureName = "veepak-golden-sample"

    func testExportLastRunPackShowsNoCompletedRunMessageWhenCaptureHasNotCompleted() {
        let coordinator = makeCaptureCoordinatorWithEmptyReplay()
        let uploader = ViewModelFakeTelemetryBatchUploader(results: [])
        let queue = TelemetryUploadQueueCoordinator(
            uploader: uploader,
            store: ViewModelInMemoryTelemetryUploadQueueStore(),
            autoUploadEnabled: false,
            enableNetworkMonitoring: false
        )
        let viewModel = makeViewModel(
            captureCoordinator: coordinator,
            uploadQueueCoordinator: queue,
            runPackStore: OBDCaptureRunPackStore(directoryURL: temporaryDirectoryURL())
        )

        viewModel.exportLastRunPack()

        XCTAssertEqual(
            viewModel.lastRunPackExportSummary,
            "No completed run available. Stop a capture first."
        )
        XCTAssertNil(viewModel.runPackShareURL)
    }

    func testRetryCountdownDisablesRetryButtonWhenUploadIsScheduled() async {
        let coordinator = makeCaptureCoordinatorWithEmptyReplay()
        let uploader = ViewModelFakeTelemetryBatchUploader(
            results: [.failure(.server(statusCode: 503, message: "service unavailable"))]
        )
        let queue = TelemetryUploadQueueCoordinator(
            uploader: uploader,
            store: ViewModelInMemoryTelemetryUploadQueueStore(),
            retryPolicy: TelemetryUploadRetryPolicy(
                maxAttempts: 3,
                initialDelaySeconds: 60,
                maxDelaySeconds: 300,
                backoffMultiplier: 2
            ),
            now: { Date(timeIntervalSince1970: 1_700_000_123) },
            autoUploadEnabled: false,
            enableNetworkMonitoring: false
        )
        let viewModel = makeViewModel(
            captureCoordinator: coordinator,
            uploadQueueCoordinator: queue,
            runPackStore: OBDCaptureRunPackStore(directoryURL: temporaryDirectoryURL())
        )

        queue.enqueue(
            batch: .sample(batchID: UUID(uuidString: "8E4FA0DF-9A83-40C0-A38E-E6166266A96A")!),
            captureWindowEndedAt: Date(timeIntervalSince1970: 1_700_000_000)
        )
        queue.triggerUpload()

        await waitUntil { viewModel.isRetryQueuedUploadsDisabled }

        XCTAssertTrue(viewModel.retryQueuedUploadsButtonTitle.contains("Retry Queued Uploads ("))
        XCTAssertNotNil(viewModel.uploadRetryCountdownText)
    }

    func testSuccessfulUploadPopulatesCopyableUploadIdentifiers() async {
        let coordinator = makeCaptureCoordinatorWithEmptyReplay()
        let batchID = UUID(uuidString: "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE")!
        let ingestID = UUID(uuidString: "12345678-1234-5678-9ABC-123456789ABC")!
        let uploader = ViewModelFakeTelemetryBatchUploader(
            results: [
                .success(
                    TelemetryBatchUploadResponse(
                        accepted: true,
                        batchID: batchID,
                        ingestID: ingestID,
                        duplicate: false,
                        recordsReceived: 0,
                        recordsAccepted: 0,
                        recordsRejected: 0,
                        errors: [],
                        nextUploadAfterSeconds: 0
                    )
                ),
            ]
        )
        let queue = TelemetryUploadQueueCoordinator(
            uploader: uploader,
            store: ViewModelInMemoryTelemetryUploadQueueStore(),
            now: { Date(timeIntervalSince1970: 1_700_000_222) },
            autoUploadEnabled: false,
            enableNetworkMonitoring: false
        )
        let viewModel = makeViewModel(
            captureCoordinator: coordinator,
            uploadQueueCoordinator: queue,
            runPackStore: OBDCaptureRunPackStore(directoryURL: temporaryDirectoryURL())
        )

        queue.enqueue(
            batch: .sample(batchID: batchID),
            captureWindowEndedAt: Date(timeIntervalSince1970: 1_700_000_000)
        )
        queue.triggerUpload()

        await waitUntil { viewModel.canCopyLastUploadIDs }

        XCTAssertTrue(viewModel.lastUploadIdentifiersSummary.contains(batchID.uuidString.lowercased()))
        XCTAssertTrue(viewModel.lastUploadIdentifiersSummary.contains(ingestID.uuidString.lowercased()))
    }

    func testExportLastRunPackCreatesShareableFileAfterReplayCapture() async throws {
        let fixture = try RunPackReplayFixture.loadCuratedFixture(named: replayFixtureName)
        let replayTransport = ReplayOBDTransport(steps: try fixture.replaySteps())
        let commandExecutor = OBDCommandExecutor(transport: replayTransport)
        let captureCoordinator = OBDTelemetryCaptureCoordinator(commandExecutor: commandExecutor)

        captureCoordinator.startCapture(sampleIntervalSeconds: 1)
        await waitUntil { captureCoordinator.pendingRecordCount >= 3 }
        captureCoordinator.stopCapture()

        let queue = TelemetryUploadQueueCoordinator(
            uploader: ViewModelFakeTelemetryBatchUploader(results: []),
            store: ViewModelInMemoryTelemetryUploadQueueStore(),
            autoUploadEnabled: false,
            enableNetworkMonitoring: false
        )
        let temporaryDirectory = temporaryDirectoryURL()
        let viewModel = makeViewModel(
            captureCoordinator: captureCoordinator,
            uploadQueueCoordinator: queue,
            runPackStore: OBDCaptureRunPackStore(directoryURL: temporaryDirectory)
        )

        viewModel.exportLastRunPack()

        XCTAssertNotNil(viewModel.runPackShareURL)
        if let runPackShareURL = viewModel.runPackShareURL {
            XCTAssertTrue(FileManager.default.fileExists(atPath: runPackShareURL.path))
        }
        XCTAssertTrue(viewModel.lastRunPackExportSummary.hasPrefix("Exported "))
    }

    private func makeCaptureCoordinatorWithEmptyReplay() -> OBDTelemetryCaptureCoordinator {
        let transport = ReplayOBDTransport(steps: [])
        let commandExecutor = OBDCommandExecutor(transport: transport)
        return OBDTelemetryCaptureCoordinator(commandExecutor: commandExecutor)
    }

    private func makeViewModel(
        captureCoordinator: OBDTelemetryCaptureCoordinator,
        uploadQueueCoordinator: TelemetryUploadQueueCoordinator,
        runPackStore: OBDCaptureRunPackStore
    ) -> OBDCaptureViewModel {
        OBDCaptureViewModel(
            bleClient: CoreBluetoothOBDClient(commandTimeoutNanoseconds: 500_000_000),
            captureCoordinator: captureCoordinator,
            uploadQueueCoordinator: uploadQueueCoordinator,
            runPackStore: runPackStore,
            sessionProvider: {
                SessionContext(
                    userID: UUID(uuidString: "e67d9de7-76a4-4f5f-8e4f-d3f1518ab8b0")!,
                    vehicleUID: UUID(uuidString: "e11889bf-504c-4238-9583-bc8840f20e19")!
                )
            },
            appVersionProvider: { "offline-viewmodel-tests" }
        )
    }

    private func temporaryDirectoryURL() -> URL {
        FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString.lowercased(), isDirectory: true)
    }

    private func waitUntil(
        timeoutSeconds: TimeInterval = 2.0,
        pollIntervalNanoseconds: UInt64 = 20_000_000,
        _ condition: @escaping () -> Bool
    ) async {
        let deadline = Date().addingTimeInterval(timeoutSeconds)
        while Date() < deadline {
            if condition() {
                return
            }
            try? await Task.sleep(nanoseconds: pollIntervalNanoseconds)
        }
        XCTFail("Timed out waiting for asynchronous condition.")
    }
}

@MainActor
private final class ViewModelFakeTelemetryBatchUploader: TelemetryBatchUploader {
    private var results: [Result<TelemetryBatchUploadResponse, BackendError>]

    init(results: [Result<TelemetryBatchUploadResponse, BackendError>]) {
        self.results = results
    }

    func upload(batch: TelemetryBatchRequest) async throws -> TelemetryBatchUploadResponse {
        guard !results.isEmpty else {
            return TelemetryBatchUploadResponse(
                accepted: true,
                batchID: batch.batchID,
                ingestID: UUID(),
                duplicate: false,
                recordsReceived: batch.records.count,
                recordsAccepted: batch.records.count,
                recordsRejected: 0,
                errors: [],
                nextUploadAfterSeconds: 0
            )
        }

        let result = results.removeFirst()
        switch result {
        case let .success(response):
            return response
        case let .failure(error):
            throw error
        }
    }
}

private final class ViewModelInMemoryTelemetryUploadQueueStore: TelemetryUploadQueueStore {
    private var queue: [TelemetryPendingBatch] = []

    func load() throws -> [TelemetryPendingBatch] {
        queue
    }

    func save(_ queue: [TelemetryPendingBatch]) throws {
        self.queue = queue
    }
}

private extension TelemetryBatchRequest {
    static func sample(batchID: UUID = UUID()) -> Self {
        TelemetryBatchRequest(
            batchID: batchID,
            vehicleUID: UUID(uuidString: "e11889bf-504c-4238-9583-bc8840f20e19")!,
            client: .init(
                appVersion: "offline-viewmodel-tests",
                adapterFingerprint: "test-adapter"
            ),
            captureWindow: .init(
                startedAt: "2026-02-21T10:00:00Z",
                endedAt: "2026-02-21T10:05:00Z",
                sampleIntervalSeconds: 60
            ),
            records: [],
            sessionEvents: [],
            diagnostics: []
        )
    }
}
