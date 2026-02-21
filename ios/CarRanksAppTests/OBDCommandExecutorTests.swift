import XCTest
@testable import CarRanksApp

@MainActor
final class OBDCommandExecutorTests: XCTestCase {
    func testBootstrapDetectsOBDLinkProfileFromATI() async throws {
        let transport = FakeOBDTransport(
            responses: [
                "ATI": "OBDLink CX v2.5",
                "ATWS": "OBDLink reset",
                "ATE0": "OK",
                "ATL0": "OK",
                "ATS0": "OK",
                "ATH0": "OK",
                "ATSP0": "OK",
                "ATAT1": "OK",
                "ATAL": "OK",
            ]
        )
        let executor = OBDCommandExecutor(transport: transport)

        try await executor.bootstrapAdapterIfNeeded()

        XCTAssertEqual(executor.detectedIdentity?.profile, .obdLink)
        XCTAssertEqual(executor.activeInitializationProfile, .obdLink)
        XCTAssertEqual(
            transport.sentCommands,
            ["ATI", "ATWS", "ATE0", "ATL0", "ATS0", "ATH0", "ATSP0", "ATAT1", "ATAL"]
        )
    }

    func testBootstrapFallsBackWhenPreferredProfileFailsValidation() async throws {
        let transport = FakeOBDTransport(
            responses: [
                "ATI": "OBDLink CX v2.5",
                "ATWS": "?", // this should fail required-response validation
                "ATZ": "ELM327 v1.5",
                "ATE0": "OK",
                "ATL0": "OK",
                "ATS0": "OK",
                "ATH0": "OK",
                "ATSP0": "OK",
                "ATAT1": "OK",
                "ATAL": "OK",
            ]
        )
        let executor = OBDCommandExecutor(transport: transport)

        try await executor.bootstrapAdapterIfNeeded()

        XCTAssertEqual(executor.detectedIdentity?.profile, .obdLink)
        XCTAssertEqual(executor.activeInitializationProfile, .elm327)
        XCTAssertEqual(
            transport.sentCommands,
            ["ATI", "ATWS", "ATZ", "ATE0", "ATL0", "ATS0", "ATH0", "ATSP0", "ATAT1", "ATAL"]
        )
    }

    func testBootstrapToleratesIdentityProbeFailure() async throws {
        let transport = FakeOBDTransport(
            responses: [
                "ATZ": "ELM327 v1.5",
                "ATE0": "OK",
                "ATL0": "OK",
                "ATS0": "OK",
                "ATH0": "OK",
                "ATSP0": "OK",
                "ATAT1": "OK",
            ],
            failingCommands: ["ATI"]
        )
        let executor = OBDCommandExecutor(transport: transport)

        try await executor.bootstrapAdapterIfNeeded()

        XCTAssertNil(executor.detectedIdentity)
        XCTAssertEqual(executor.activeInitializationProfile, .generic)
        XCTAssertEqual(transport.sentCommands.first, "ATI")
    }

    func testBootstrapIgnoresOptionalCommandFailures() async throws {
        let transport = FakeOBDTransport(
            responses: [
                "ATI": "ELM327 v1.5",
                "ATZ": "ELM327 v1.5",
                "ATE0": "OK",
                "ATL0": "OK",
                "ATS0": "OK",
                "ATH0": "OK",
                "ATSP0": "OK",
            ],
            failingCommands: ["ATAT1", "ATAL"]
        )
        let executor = OBDCommandExecutor(transport: transport)

        try await executor.bootstrapAdapterIfNeeded()

        XCTAssertEqual(executor.activeInitializationProfile, .elm327)
    }

    func testBootstrapSendsInitializationCommandsOnlyOnce() async throws {
        let transport = FakeOBDTransport(
            responses: [
                "ATI": "ELM327 v1.5",
                "ATZ": "ELM327 v1.5",
                "ATE0": "OK",
                "ATL0": "OK",
                "ATS0": "OK",
                "ATH0": "OK",
                "ATSP0": "OK",
                "ATAT1": "OK",
                "ATAL": "OK",
            ]
        )
        let executor = OBDCommandExecutor(transport: transport)

        try await executor.bootstrapAdapterIfNeeded()
        let commandCountAfterFirstBootstrap = transport.sentCommands.count
        try await executor.bootstrapAdapterIfNeeded()

        XCTAssertEqual(transport.sentCommands.count, commandCountAfterFirstBootstrap)
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
            failingCommands: ["010D"]
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
    private let failingCommands: Set<String>

    private(set) var sentCommands: [String] = []

    init(
        responses: [String: String],
        failingCommands: Set<String> = []
    ) {
        self.responses = responses
        self.failingCommands = failingCommands
    }

    func startScanning() {}
    func stopScanning() {}
    func connect(to _: UUID) {}
    func disconnect() {}

    func sendRawCommand(_ command: String) async throws -> String {
        sentCommands.append(command)

        if failingCommands.contains(command) {
            throw BackendError.transport("forced failure for \(command)")
        }

        guard let response = responses[command] else {
            throw BackendError.transport("missing fake response for \(command)")
        }
        return response
    }
}
