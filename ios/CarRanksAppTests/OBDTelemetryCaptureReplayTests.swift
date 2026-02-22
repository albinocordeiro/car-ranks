import XCTest
@testable import CarRanksApp

@MainActor
final class OBDTelemetryCaptureReplayTests: XCTestCase {
    private let curatedFixtureName = "veepak-golden-sample"

    func testReplayCaptureBuildsBatchWithSessionCorrelation() async throws {
        let fixture = try RunPackReplayFixture.loadCuratedFixture(named: curatedFixtureName)
        let transport = ReplayOBDTransport(steps: try fixture.replaySteps())
        let executor = OBDCommandExecutor(transport: transport)
        let coordinator = OBDTelemetryCaptureCoordinator(commandExecutor: executor)

        coordinator.startCapture(sampleIntervalSeconds: 1)
        try await waitForCondition {
            coordinator.pendingRecordCount >= 3 && coordinator.pendingSessionEventCount >= 1
        }
        coordinator.stopCapture()

        let vehicleUID = UUID(uuidString: "e11889bf-504c-4238-9583-bc8840f20e19")!
        let batchBundle = coordinator.buildBatch(
            vehicleUID: vehicleUID,
            appVersion: "offline-replay-tests",
            adapterFingerprint: "replay-obd-adapter"
        )

        XCTAssertNotNil(batchBundle)
        guard let batch = batchBundle?.request else {
            return
        }

        XCTAssertFalse(batch.records.isEmpty)
        let sessionIDs = Set(batch.records.compactMap(\.sessionID))
        XCTAssertEqual(sessionIDs.count, 1)
        XCTAssertEqual(batch.sessionEvents.count, 2)
        XCTAssertEqual(batch.sessionEvents.first?.eventType, "drive_session_start")
        XCTAssertEqual(batch.sessionEvents.last?.eventType, "drive_session_stop")

        let voltageRecord = batch.records.first { $0.signalKey == "power.battery_voltage" }
        XCTAssertEqual(voltageRecord?.sourceSignal, "AT_RV")
    }

    func testReplayCaptureRecordsCommandExchangesForRunPackExport() async throws {
        let fixture = try RunPackReplayFixture.loadCuratedFixture(named: curatedFixtureName)
        let transport = ReplayOBDTransport(steps: try fixture.replaySteps())
        let executor = OBDCommandExecutor(transport: transport)
        let coordinator = OBDTelemetryCaptureCoordinator(commandExecutor: executor)

        coordinator.startCapture(sampleIntervalSeconds: 1)
        try await waitForCondition { coordinator.commandExchangeCount >= 10 }
        coordinator.stopCapture()

        let sessionContext = SessionContext(
            userID: UUID(uuidString: "e67d9de7-76a4-4f5f-8e4f-d3f1518ab8b0")!,
            vehicleUID: UUID(uuidString: "e11889bf-504c-4238-9583-bc8840f20e19")!
        )

        let runPack = coordinator.buildLastRunPack(
            sessionContext: sessionContext,
            appVersion: "offline-replay-tests",
            adapterFingerprint: "replay-obd-adapter",
            uploadReceipt: nil
        )

        XCTAssertNotNil(runPack)
        XCTAssertEqual(runPack?.sessionID, coordinator.lastCompletedRunSessionID)
        XCTAssertTrue((runPack?.commandExchanges.count ?? 0) >= 10)
    }

    private func waitForCondition(
        timeoutSeconds: TimeInterval = 2.0,
        pollIntervalNanoseconds: UInt64 = 20_000_000,
        _ condition: @escaping () -> Bool
    ) async throws {
        let deadline = Date().addingTimeInterval(timeoutSeconds)
        while Date() < deadline {
            if condition() {
                return
            }
            try await Task.sleep(nanoseconds: pollIntervalNanoseconds)
        }
        XCTFail("Timed out waiting for replay capture condition.")
    }
}
