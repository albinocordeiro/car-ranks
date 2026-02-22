import XCTest
@testable import CarRanksApp

@MainActor
final class OBDCommandExecutorReplayTests: XCTestCase {
    private let curatedFixtureName = "veepak-golden-sample"

    func testReplayTransportRejectsOutOfOrderCommands() async {
        let transport = ReplayOBDTransport(
            steps: [
                ReplayCommandStep(command: "010D", outcome: .response("41 0D 14")),
            ]
        )

        do {
            _ = try await transport.sendRawCommand("0142")
            XCTFail("Expected a command-order mismatch error.")
        } catch {
            let message = OBDErrorPresentation.message(from: error)
            XCTAssertTrue(message.contains("expected 010D"))
            XCTAssertTrue(message.contains("got 0142"))
        }
    }

    func testReplayFixturePreservesFallbackFlowForControlModuleVoltage() async throws {
        let fixture = try RunPackReplayFixture.loadCuratedFixture(named: curatedFixtureName)
        let transport = ReplayOBDTransport(
            steps: try fixture.replaySteps(startingAt: "0142")
        )
        let executor = OBDCommandExecutor(transport: transport)

        let record = await executor.poll(signal: .controlModuleVoltage, observedAt: .distantPast)

        XCTAssertEqual(record.status, .ok)
        XCTAssertEqual(record.sourceSignal, "AT_RV")
        XCTAssertEqual(record.valueNumber ?? 0, 12.8, accuracy: 0.001)
        XCTAssertEqual(transport.sentCommands, ["0142", "ATRV"])
    }

    func testReplayFixtureCanSimulateTimeoutErrors() async throws {
        let fixture = try RunPackReplayFixture.loadCuratedFixture(named: curatedFixtureName)
        let transport = ReplayOBDTransport(
            steps: try fixture.replaySteps(startingAt: "010D", occurrence: 2)
        )
        let executor = OBDCommandExecutor(transport: transport)

        let record = await executor.poll(signal: .vehicleSpeed, observedAt: .distantPast)

        XCTAssertEqual(record.status, .error)
        XCTAssertTrue(record.rawPayloadRef?.contains("timeout while waiting for response") == true)
        XCTAssertEqual(transport.sentCommands, ["010D"])
    }

    func testReplayFixtureCanSimulateMalformedParsePayload() async throws {
        let fixture = try RunPackReplayFixture.loadCuratedFixture(named: curatedFixtureName)
        let transport = ReplayOBDTransport(
            steps: try fixture.replaySteps(startingAt: "010D", occurrence: 3)
        )
        let executor = OBDCommandExecutor(transport: transport)

        let record = await executor.poll(signal: .vehicleSpeed, observedAt: .distantPast)

        XCTAssertEqual(record.status, .unavailable)
        XCTAssertNil(record.valueNumber)
        XCTAssertEqual(transport.sentCommands, ["010D"])
    }
}
