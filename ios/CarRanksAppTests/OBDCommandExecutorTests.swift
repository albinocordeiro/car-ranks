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
                "ATSP6": "OK",
                "ATAT1": "OK",
                "ATAL": "OK",
                "0100": "41 00 00 08 00 00",
                "0140": "41 40 44 00 00 00",
            ]
        )
        let executor = OBDCommandExecutor(transport: transport)

        try await executor.bootstrapAdapterIfNeeded()

        XCTAssertEqual(executor.detectedIdentity?.profile, .obdLink)
        XCTAssertEqual(executor.activeInitializationProfile, .obdLink)
        XCTAssertEqual(
            transport.sentCommands,
            [
                "ATI", "ATWS", "ATE0", "ATL0", "ATS0", "ATH0", "ATSP0", "ATAT1", "ATAL", "0100",
                "0140", "ATDP",
            ]
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
                "ATSP6": "OK",
                "ATAT1": "OK",
                "ATAL": "OK",
                "0100": "41 00 00 08 00 00",
                "0140": "41 40 44 00 00 00",
            ]
        )
        let executor = OBDCommandExecutor(transport: transport)

        try await executor.bootstrapAdapterIfNeeded()

        XCTAssertEqual(executor.detectedIdentity?.profile, .obdLink)
        XCTAssertEqual(executor.activeInitializationProfile, .elm327)
        XCTAssertEqual(
            transport.sentCommands,
            [
                "ATI", "ATWS", "ATZ", "ATE0", "ATL0", "ATS0", "ATH0", "ATSP0", "ATAT1", "ATAL",
                "ATSP6", "0100", "0140", "ATDP",
            ]
        )
    }

    func testBootstrapFallsBackToAutoProtocolWhenFixedCanProbeFails() async throws {
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
                "0100": "41 00 00 08 00 00",
                "0140": "41 40 44 00 00 00",
            ],
            failingCommands: ["ATSP6"]
        )
        let executor = OBDCommandExecutor(transport: transport)

        try await executor.bootstrapAdapterIfNeeded()

        XCTAssertEqual(executor.activeInitializationProfile, .elm327)
        XCTAssertTrue(transport.sentCommands.contains("ATSP6"))
        XCTAssertEqual(
            transport.sentCommands.filter { $0 == "ATSP0" }.count,
            2,
            "Initializer should retry auto protocol after forced CAN command failure."
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
                "0100": "41 00 00 08 00 00",
                "0140": "41 40 44 00 00 00",
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
                "0100": "41 00 00 08 00 00",
                "0140": "41 40 44 00 00 00",
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
                "0100": "41 00 00 08 00 00",
                "0140": "41 40 44 00 00 00",
            ]
        )
        let executor = OBDCommandExecutor(transport: transport)

        try await executor.bootstrapAdapterIfNeeded()
        let commandCountAfterFirstBootstrap = transport.sentCommands.count
        try await executor.bootstrapAdapterIfNeeded()

        XCTAssertEqual(transport.sentCommands.count, commandCountAfterFirstBootstrap)
    }

    func testPollReturnsNotSupportedWhenProbeMarksPidUnsupported() async throws {
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
                // Only speed (`0x0D`) is marked as supported.
                "0100": "41 00 00 08 00 00",
                // No supported PIDs in `0x41...0x60`.
                "0140": "41 40 00 00 00 00",
            ]
        )
        let executor = OBDCommandExecutor(transport: transport)
        try await executor.bootstrapAdapterIfNeeded()

        let record = await executor.poll(signal: .ambientTemperature)

        XCTAssertEqual(record.status, .notSupported)
        XCTAssertTrue(record.rawPayloadRef?.contains("unsupported_pid") == true)
        XCTAssertFalse(transport.sentCommands.contains("0146"))
    }

    func testPollFallsBackToNormalPollingWhenProbeBlockCannotBeDecoded() async throws {
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
                "0100": "41 00 00 08 00 00",
                // Purposefully omit `0140`; executor should keep polling this block.
                "0146": "41 46 22",
            ]
        )
        let executor = OBDCommandExecutor(transport: transport)
        try await executor.bootstrapAdapterIfNeeded()

        let record = await executor.poll(signal: .ambientTemperature)

        XCTAssertEqual(record.status, .ok)
        XCTAssertEqual(record.valueNumber, -6)
        XCTAssertTrue(transport.sentCommands.contains("0146"))
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
        XCTAssertEqual(record.rawPayloadRef, "cmd=010D resp=41 0D 14")
    }

    func testPollControlModuleVoltageFallsBackToATRVWhenPidIsUnavailable() async {
        let transport = FakeOBDTransport(
            responses: [
                "0142": "NO DATA",
                "ATRV": "12.8V",
            ]
        )
        let executor = OBDCommandExecutor(transport: transport)

        let record = await executor.poll(signal: .controlModuleVoltage, observedAt: .distantPast)

        XCTAssertEqual(record.signalKey, "power.battery_voltage")
        XCTAssertEqual(record.status, .ok)
        XCTAssertEqual(record.valueNumber ?? 0, 12.8, accuracy: 0.001)
        XCTAssertEqual(record.sourceSignal, "AT_RV")
        XCTAssertTrue(record.rawPayloadRef?.contains("fallback=ATRV") == true)
        XCTAssertEqual(transport.sentCommands, ["0142", "ATRV"])
    }

    func testPollControlModuleVoltageDoesNotFallbackWhenPidDecodes() async {
        let transport = FakeOBDTransport(
            responses: [
                "0142": "41 42 0D 9A",
                "ATRV": "12.8V",
            ]
        )
        let executor = OBDCommandExecutor(transport: transport)

        let record = await executor.poll(signal: .controlModuleVoltage, observedAt: .distantPast)

        XCTAssertEqual(record.status, .ok)
        XCTAssertEqual(record.valueNumber ?? 0, 3.482, accuracy: 0.001)
        XCTAssertEqual(record.sourceSignal, "01_42")
        XCTAssertEqual(transport.sentCommands, ["0142"])
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
        XCTAssertTrue(record.rawPayloadRef?.contains("cmd=010D error=") == true)
    }

    func testPollDiagnosticsReturnsMilAndStoredCodes() async {
        let transport = FakeOBDTransport(
            responses: [
                "0101": "41 01 82 00 00 00",
                "03": "43 01 0A C1 23 00 00",
            ]
        )
        let executor = OBDCommandExecutor(transport: transport)

        let snapshot = await executor.pollDiagnostics(observedAt: .distantPast)

        XCTAssertEqual(snapshot?.milOn, true)
        XCTAssertEqual(snapshot?.dtcsActive, ["P010A", "U0123"])
    }

    func testPollDiagnosticsReturnsMilOnlyWhenDtcQueryFails() async {
        let transport = FakeOBDTransport(
            responses: [
                "0101": "41 01 80 00 00 00",
            ],
            failingCommands: ["03"]
        )
        let executor = OBDCommandExecutor(transport: transport)

        let snapshot = await executor.pollDiagnostics(observedAt: .distantPast)

        XCTAssertEqual(snapshot?.milOn, true)
        XCTAssertEqual(snapshot?.dtcsActive, [])
    }

    func testPollDiagnosticsReturnsNilWhenReadinessCannotBeDecoded() async {
        let transport = FakeOBDTransport(
            responses: [
                "0101": "NO DATA",
            ]
        )
        let executor = OBDCommandExecutor(transport: transport)

        let snapshot = await executor.pollDiagnostics(observedAt: .distantPast)
        XCTAssertNil(snapshot)
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
