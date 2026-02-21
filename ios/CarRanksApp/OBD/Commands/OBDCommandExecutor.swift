import Foundation

/// Serial command workflow that initializes adapters and polls standard OBD signals.
@MainActor
final class OBDCommandExecutor {
    private let transport: OBDBLETransport
    private var didBootstrapAdapter = false

    /// Captured for debugging and telemetry diagnostics when adapter compatibility issues arise.
    private(set) var detectedIdentity: OBDAdapterIdentity?
    private(set) var activeInitializationProfile: OBDAdapterInitializationProfile?

    init(transport: OBDBLETransport) {
        self.transport = transport
    }

    func bootstrapAdapterIfNeeded() async throws {
        guard !didBootstrapAdapter else { return }

        detectedIdentity = try? await probeAdapterIdentity()
        let plans = OBDAdapterInitializationProfile.fallbackOrder(preferred: detectedIdentity?.profile)

        var lastError: Error = BackendError.transport("OBD adapter initialization failed before commands were sent.")
        for profile in plans {
            do {
                try await runInitializationPlan(for: profile)
                activeInitializationProfile = profile
                didBootstrapAdapter = true
                return
            } catch {
                lastError = error
            }
        }

        throw lastError
    }

    func resetBootstrapState() {
        didBootstrapAdapter = false
        detectedIdentity = nil
        activeInitializationProfile = nil
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

    private func probeAdapterIdentity() async throws -> OBDAdapterIdentity {
        let response = try await transport.sendRawCommand("ATI")
        return OBDAdapterIdentity.fromATIResponse(response)
    }

    private func runInitializationPlan(for profile: OBDAdapterInitializationProfile) async throws {
        let steps = OBDInitializationPlan.steps(for: profile)
        for step in steps {
            do {
                let rawResponse = try await transport.sendRawCommand(step.command)
                if step.isRequired {
                    try validateRequiredResponse(rawResponse, forCommand: step.command)
                }
            } catch {
                if step.isRequired {
                    throw error
                }
            }
        }
    }

    private func validateRequiredResponse(_ rawResponse: String, forCommand command: String) throws {
        let normalized = normalize(rawResponse)
        guard !normalized.isEmpty else {
            throw BackendError.transport("Adapter command \(command) returned an empty response.")
        }

        let uppercased = normalized.uppercased()
        let knownFailureTokens = [
            "UNABLE TO CONNECT",
            "NO CARRIER",
            "ERROR",
            "STOPPED",
            "?",
        ]
        if knownFailureTokens.contains(where: { uppercased.contains($0) }) {
            throw BackendError.transport("Adapter command \(command) failed with '\(normalized)'.")
        }
    }

    private func normalize(_ rawResponse: String) -> String {
        rawResponse
            .replacingOccurrences(of: ">", with: " ")
            .components(separatedBy: .newlines)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
            .joined(separator: " ")
    }
}
