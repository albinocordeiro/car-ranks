import XCTest
@testable import CarRanksApp

@MainActor
final class OBDTelemetryCaptureCoordinatorTests: XCTestCase {
    func testFailedBootstrapDoesNotAccumulateSessionEventsAcrossRetries() async {
        let transport = FailingBootstrapOBDTransport()
        let executor = OBDCommandExecutor(transport: transport)
        let coordinator = OBDTelemetryCaptureCoordinator(commandExecutor: executor)

        coordinator.startCapture(sampleIntervalSeconds: 5)
        await waitUntilCaptureStops(coordinator)

        XCTAssertFalse(coordinator.isCapturing)
        XCTAssertEqual(coordinator.pendingSessionEventCount, 0)
        XCTAssertEqual(coordinator.pendingRecordCount, 0)
        XCTAssertEqual(coordinator.pendingDiagnosticCount, 0)
        XCTAssertEqual(
            coordinator.lastCaptureError,
            "Capture failed to start: forced bootstrap failure"
        )

        coordinator.startCapture(sampleIntervalSeconds: 5)
        await waitUntilCaptureStops(coordinator)

        XCTAssertEqual(coordinator.pendingSessionEventCount, 0)
        XCTAssertEqual(coordinator.pendingRecordCount, 0)
        XCTAssertEqual(coordinator.pendingDiagnosticCount, 0)
    }

    func testSuccessfulCaptureAppendsOneStartAndOneStopSessionEvent() async {
        let transport = SuccessfulBootstrapOBDTransport()
        let executor = OBDCommandExecutor(transport: transport)
        let coordinator = OBDTelemetryCaptureCoordinator(commandExecutor: executor)

        coordinator.startCapture(sampleIntervalSeconds: 5)
        await waitUntilSessionEventCount(atLeast: 1, coordinator: coordinator)

        XCTAssertTrue(coordinator.isCapturing)
        XCTAssertEqual(coordinator.pendingSessionEventCount, 1)

        coordinator.stopCapture()
        XCTAssertFalse(coordinator.isCapturing)
        XCTAssertEqual(coordinator.pendingSessionEventCount, 2)
    }

    func testBatchPayloadClampsSampleIntervalToBackendMinimum() async {
        let transport = SuccessfulBootstrapOBDTransport()
        let executor = OBDCommandExecutor(transport: transport)
        let coordinator = OBDTelemetryCaptureCoordinator(commandExecutor: executor)

        coordinator.startCapture(sampleIntervalSeconds: 5)
        await waitUntilSessionEventCount(atLeast: 1, coordinator: coordinator)
        coordinator.stopCapture()

        let batchBundle = coordinator.buildBatch(
            vehicleUID: UUID(uuidString: "e11889bf-504c-4238-9583-bc8840f20e19")!,
            appVersion: "test",
            adapterFingerprint: "fake-fingerprint"
        )

        XCTAssertNotNil(batchBundle)
        XCTAssertEqual(batchBundle?.request.captureWindow.sampleIntervalSeconds, 60)
    }

    func testBatchPayloadIncludesBootstrapSummaryOnDriveStartSessionEvent() async {
        let transport = SuccessfulBootstrapOBDTransport()
        let executor = OBDCommandExecutor(transport: transport)
        let coordinator = OBDTelemetryCaptureCoordinator(commandExecutor: executor)

        coordinator.startCapture(sampleIntervalSeconds: 5)
        await waitUntilSessionEventCount(atLeast: 1, coordinator: coordinator)
        coordinator.stopCapture()

        let batchBundle = coordinator.buildBatch(
            vehicleUID: UUID(uuidString: "e11889bf-504c-4238-9583-bc8840f20e19")!,
            appVersion: "test",
            adapterFingerprint: "fake-fingerprint"
        )

        let driveStartEvent = batchBundle?.request.sessionEvents.first(where: {
            $0.eventType == "drive_session_start"
        })
        XCTAssertNotNil(driveStartEvent)
        XCTAssertTrue(driveStartEvent?.rawPayloadRef?.contains("profile=elm327") == true)
        XCTAssertTrue(
            driveStartEvent?.rawPayloadRef?.contains("ATDP raw=AUTO, ISO 15765-4 (CAN 11/500)")
                == true
        )
    }

    func testBatchPayloadIncludesSessionIDOnSignalRecords() async {
        let transport = SuccessfulBootstrapOBDTransport()
        let executor = OBDCommandExecutor(transport: transport)
        let coordinator = OBDTelemetryCaptureCoordinator(commandExecutor: executor)

        coordinator.startCapture(sampleIntervalSeconds: 5)
        await waitUntilSessionEventCount(atLeast: 1, coordinator: coordinator)
        coordinator.stopCapture()

        let batchBundle = coordinator.buildBatch(
            vehicleUID: UUID(uuidString: "e11889bf-504c-4238-9583-bc8840f20e19")!,
            appVersion: "test",
            adapterFingerprint: "fake-fingerprint"
        )

        let driveStartEvent = batchBundle?.request.sessionEvents.first(where: {
            $0.eventType == "drive_session_start"
        })
        let signalSessionIDs = Set(batchBundle?.request.records.compactMap(\.sessionID) ?? [])
        XCTAssertEqual(signalSessionIDs.count, 1)
        XCTAssertEqual(signalSessionIDs.first, driveStartEvent?.sessionID)
    }

    private func waitUntilCaptureStops(
        _ coordinator: OBDTelemetryCaptureCoordinator,
        timeoutNanoseconds: UInt64 = 1_500_000_000
    ) async {
        let deadline = DispatchTime.now().uptimeNanoseconds + timeoutNanoseconds
        while coordinator.isCapturing, DispatchTime.now().uptimeNanoseconds < deadline {
            try? await Task.sleep(nanoseconds: 20_000_000)
        }
        if coordinator.isCapturing {
            XCTFail("Capture did not stop before timeout.")
        }
    }

    private func waitUntilSessionEventCount(
        atLeast expectedMinimum: Int,
        coordinator: OBDTelemetryCaptureCoordinator,
        timeoutNanoseconds: UInt64 = 1_500_000_000
    ) async {
        let deadline = DispatchTime.now().uptimeNanoseconds + timeoutNanoseconds
        while coordinator.pendingSessionEventCount < expectedMinimum,
              DispatchTime.now().uptimeNanoseconds < deadline
        {
            try? await Task.sleep(nanoseconds: 20_000_000)
        }
        if coordinator.pendingSessionEventCount < expectedMinimum {
            XCTFail("Session events did not reach expected minimum before timeout.")
        }
    }
}

@MainActor
private final class FailingBootstrapOBDTransport: OBDBLETransport {
    var discoveredDevices: [OBDAdapterDevice] = []
    var connectionState: OBDConnectionState = .connected("Fake Adapter")
    var adapterFingerprint: String? = "fake-fingerprint"

    func startScanning() {}
    func stopScanning() {}
    func connect(to _: UUID) {}
    func disconnect() {}

    func sendRawCommand(_: String) async throws -> String {
        throw BackendError.transport("forced bootstrap failure")
    }
}

@MainActor
private final class SuccessfulBootstrapOBDTransport: OBDBLETransport {
    var discoveredDevices: [OBDAdapterDevice] = []
    var connectionState: OBDConnectionState = .connected("Fake Adapter")
    var adapterFingerprint: String? = "fake-fingerprint"

    private let responses: [String: String] = [
        "ATI": "ELM327 v1.5",
        "ATZ": "ELM327 v1.5",
        "ATE0": "OK",
        "ATL0": "OK",
        "ATS0": "OK",
        "ATH0": "OK",
        "ATSP0": "OK",
        "ATAT1": "OK",
        "ATAL": "OK",
        "0100": "41 00 00 08 00 00",
        "0140": "41 40 44 00 00 00",
        "ATDP": "AUTO, ISO 15765-4 (CAN 11/500)",
        "0101": "41 01 00 00 00 00",
        "03": "43 00 00 00 00 00",
    ]

    func startScanning() {}
    func stopScanning() {}
    func connect(to _: UUID) {}
    func disconnect() {}

    func sendRawCommand(_ command: String) async throws -> String {
        if let response = responses[command] {
            return response
        }

        // Unknown polling commands are treated as temporary adapter misses.
        throw BackendError.transport("forced polling failure for \(command)")
    }
}
