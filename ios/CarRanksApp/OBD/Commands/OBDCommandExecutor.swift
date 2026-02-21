import Foundation

/// Serial command workflow for ELM-compatible adapters.
@MainActor
final class OBDCommandExecutor {
    private let transport: OBDBLETransport
    private var didBootstrapAdapter = false

    init(transport: OBDBLETransport) {
        self.transport = transport
    }

    func bootstrapAdapterIfNeeded() async throws {
        guard !didBootstrapAdapter else { return }

        // Keep initialization explicit to make adapter compatibility issues easy to debug.
        let initCommands = [
            "ATZ", // reset adapter
            "ATE0", // echo off
            "ATL0", // linefeeds off
            "ATS0", // spaces off
            "ATH0", // headers off
            "ATSP0", // auto protocol detection
        ]

        for command in initCommands {
            _ = try await transport.sendRawCommand(command)
        }
        didBootstrapAdapter = true
    }

    func resetBootstrapState() {
        didBootstrapAdapter = false
    }

    func poll(signal: OBDStandardSignal, observedAt: Date = Date()) async -> OBDSignalRecord {
        do {
            let rawResponse = try await transport.sendRawCommand(signal.command)
            guard let decodedValue = signal.decodeValue(from: rawResponse) else {
                return OBDSignalRecord(
                    observedAt: observedAt,
                    signalKey: signal.signalKey,
                    valueNumber: nil,
                    unit: signal.unit,
                    status: .unavailable,
                    confidence: nil,
                    sourceSignal: signal.sourceSignal
                )
            }

            return OBDSignalRecord(
                observedAt: observedAt,
                signalKey: signal.signalKey,
                valueNumber: decodedValue,
                unit: signal.unit,
                status: .ok,
                confidence: signal.confidence,
                sourceSignal: signal.sourceSignal
            )
        } catch {
            return OBDSignalRecord(
                observedAt: observedAt,
                signalKey: signal.signalKey,
                valueNumber: nil,
                unit: signal.unit,
                status: .error,
                confidence: nil,
                sourceSignal: signal.sourceSignal
            )
        }
    }
}
