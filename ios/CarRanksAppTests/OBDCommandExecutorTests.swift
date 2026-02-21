import XCTest
@testable import CarRanksApp

@MainActor
final class OBDCommandExecutorTests: XCTestCase {
    func testBootstrapSendsInitializationCommandsOnlyOnce() async throws {
        let transport = FakeOBDTransport(
            responses: [
                "ATZ": "ELM327 v1.5",
                "ATE0": "OK",
                "ATL0": "OK",
                "ATS0": "OK",
                "ATH0": "OK",
                "ATSP0": "OK",
            ]
        )
        let executor = OBDCommandExecutor(transport: transport)

        try await executor.bootstrapAdapterIfNeeded()
        try await executor.bootstrapAdapterIfNeeded()

        XCTAssertEqual(
            transport.sentCommands,
            ["ATZ", "ATE0", "ATL0", "ATS0", "ATH0", "ATSP0"]
        )
    }

    func testPollReturnsOkWhenSignalCanBeDecoded() async {
        let transport = FakeOBDTransport(
            responses: [
                "010D": "41 0D 14",
            ]
        )
        let executor = OBDCommandExecutor(transport: transport)

        let record = await executor.poll(signal: .vehicleSpeed, observedAt: .distantPast)

        XCTAssertEqual(record.signalKey, "speed.vehicle")
        XCTAssertEqual(record.status, .ok)
        XCTAssertEqual(record.valueNumber, 20)
    }

    func testPollReturnsErrorWhenTransportThrows() async {
        let transport = FakeOBDTransport(
            responses: [:],
            errorMessage: "timeout"
        )
        let executor = OBDCommandExecutor(transport: transport)

        let record = await executor.poll(signal: .vehicleSpeed)

        XCTAssertEqual(record.status, .error)
        XCTAssertNil(record.valueNumber)
    }
}

@MainActor
private final class FakeOBDTransport: OBDBLETransport {
    var discoveredDevices: [OBDAdapterDevice] = []
    var connectionState: OBDConnectionState = .connected("fake")
    var adapterFingerprint: String? = "fake-fingerprint"

    private let responses: [String: String]
    private let errorMessage: String?

    private(set) var sentCommands: [String] = []

    init(responses: [String: String], errorMessage: String? = nil) {
        self.responses = responses
        self.errorMessage = errorMessage
    }

    func startScanning() {}
    func stopScanning() {}
    func connect(to _: UUID) {}
    func disconnect() {}

    func sendRawCommand(_ command: String) async throws -> String {
        sentCommands.append(command)

        if let errorMessage {
            throw BackendError.transport(errorMessage)
        }

        guard let response = responses[command] else {
            throw BackendError.transport("missing fake response")
        }
        return response
    }
}
