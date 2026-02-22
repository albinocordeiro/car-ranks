import Foundation
@testable import CarRanksApp

/// Deterministic, script-driven OBD transport used for offline replay tests.
///
/// Each `sendRawCommand` call consumes exactly one scripted step. If command order diverges,
/// the transport throws immediately so tests can pinpoint sequencing regressions.
@MainActor
final class ReplayOBDTransport: OBDBLETransport {
    var discoveredDevices: [OBDAdapterDevice] = []
    var connectionState: OBDConnectionState = .connected("Replay adapter")
    var adapterFingerprint: String?

    private let steps: [ReplayCommandStep]
    private(set) var sentCommands: [String] = []
    private var cursor = 0

    init(
        steps: [ReplayCommandStep],
        adapterFingerprint: String? = "replay-obd-adapter"
    ) {
        self.steps = steps
        self.adapterFingerprint = adapterFingerprint
    }

    func startScanning() {}
    func stopScanning() {}
    func connect(to _: UUID) {}
    func disconnect() {}

    func sendRawCommand(_ command: String) async throws -> String {
        sentCommands.append(command)

        guard cursor < steps.count else {
            throw BackendError.transport(
                "Replay exhausted: no scripted step available for command \(command)."
            )
        }

        let expectedStep = steps[cursor]
        guard expectedStep.command.caseInsensitiveCompare(command) == .orderedSame else {
            throw BackendError.transport(
                "Replay command mismatch at step \(cursor + 1): expected \(expectedStep.command), got \(command)."
            )
        }

        cursor += 1
        switch expectedStep.outcome {
        case let .response(response):
            return response
        case let .transportError(message):
            throw BackendError.transport(message)
        case let .timeout(message):
            throw BackendError.transport(message)
        }
    }
}
