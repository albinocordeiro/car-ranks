import Foundation

/// Serial command workflow that initializes adapters and polls standard OBD signals.
@MainActor
final class OBDCommandExecutor {
    private let transport: OBDBLETransport
    private var didBootstrapAdapter = false
    private let maxRawPayloadRefLength = 240
    private let supportProbeAttemptsPerBlock = 3
    private let supportProbeRetryDelayNanoseconds: UInt64 = 250_000_000
    private var supportedMode1PidsByBlock: [UInt8: Set<UInt8>] = [:]
    private var commandExchangeObserver: ((OBDCommandExchange) -> Void)?

    /// Captured for debugging and telemetry diagnostics when adapter compatibility issues arise.
    private(set) var detectedIdentity: OBDAdapterIdentity?
    private(set) var activeInitializationProfile: OBDAdapterInitializationProfile?
    private(set) var bootstrapSessionEventPayloadRef: String?

    init(
        transport: OBDBLETransport,
        commandExchangeObserver: ((OBDCommandExchange) -> Void)? = nil
    ) {
        self.transport = transport
        self.commandExchangeObserver = commandExchangeObserver
    }

    /// Coordinator-level hook used to collect full command traces into run-pack artifacts.
    func setCommandExchangeObserver(_ observer: ((OBDCommandExchange) -> Void)?) {
        commandExchangeObserver = observer
    }

    func bootstrapAdapterIfNeeded() async throws {
        guard !didBootstrapAdapter else { return }

        detectedIdentity = try? await probeAdapterIdentity()
        let plans = OBDAdapterInitializationProfile.fallbackOrder(preferred: detectedIdentity?.profile)

        var lastError: Error = BackendError.transport("OBD adapter initialization failed before commands were sent.")
        for profile in plans {
            do {
                try await runInitializationPlan(for: profile)
                let supportProbe = await probeSupportedMode1PidsWithProtocolFallback(
                    for: OBDStandardSignal.allCases,
                    profile: profile
                )
                supportedMode1PidsByBlock = supportProbe.supportByBlock
                activeInitializationProfile = profile
                bootstrapSessionEventPayloadRef = await buildBootstrapSessionEventPayloadRef(
                    activeProfile: profile,
                    supportProbeSummaries: supportProbe.summarySegments
                )
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
        supportedMode1PidsByBlock = [:]
        bootstrapSessionEventPayloadRef = nil
    }

    func poll(signal: OBDStandardSignal, observedAt: Date = Date()) async -> OBDSignalRecord {
        if isSignalExplicitlyUnsupported(signal) {
            emitCommandExchange(
                startedAt: observedAt,
                endedAt: observedAt,
                command: signal.command,
                rawResponse: nil,
                errorMessage: nil,
                parseOutcome: .notSupported,
                signalKey: signal.signalKey,
                sourceSignal: signal.sourceSignal
            )
            return OBDSignalRecord(
                observedAt: observedAt,
                signalKey: signal.signalKey,
                valueNumber: nil,
                unit: signal.unit,
                status: .notSupported,
                confidence: nil,
                sourceSignal: signal.sourceSignal,
                rawPayloadRef: buildUnsupportedPayloadRef(signal: signal)
            )
        }

        do {
            let commandResult = try await sendRawCommandWithTiming(
                signal.command,
                signalKey: signal.signalKey,
                sourceSignal: signal.sourceSignal
            )
            let rawResponse = commandResult.rawResponse
            let rawPayloadRef = buildRawPayloadRef(command: signal.command, rawResponse: rawResponse)
            if let decodedValue = signal.decodeValue(from: rawResponse) {
                emitCommandExchange(
                    startedAt: commandResult.startedAt,
                    endedAt: commandResult.endedAt,
                    command: signal.command,
                    rawResponse: rawResponse,
                    errorMessage: nil,
                    parseOutcome: .ok,
                    signalKey: signal.signalKey,
                    sourceSignal: signal.sourceSignal
                )
                return OBDSignalRecord(
                    observedAt: observedAt,
                    signalKey: signal.signalKey,
                    valueNumber: decodedValue,
                    unit: signal.unit,
                    status: .ok,
                    confidence: signal.confidence,
                    sourceSignal: signal.sourceSignal,
                    rawPayloadRef: rawPayloadRef
                )
            }

            if signal == .controlModuleVoltage,
               let fallbackRecord = await pollControlModuleVoltageFallback(
                   signal: signal,
                   observedAt: observedAt,
                   primaryResponse: rawResponse,
                   primaryCommandStartedAt: commandResult.startedAt,
                   primaryCommandEndedAt: commandResult.endedAt
               )
            {
                return fallbackRecord
            }

            emitCommandExchange(
                startedAt: commandResult.startedAt,
                endedAt: commandResult.endedAt,
                command: signal.command,
                rawResponse: rawResponse,
                errorMessage: nil,
                parseOutcome: .unavailable,
                signalKey: signal.signalKey,
                sourceSignal: signal.sourceSignal
            )

            return OBDSignalRecord(
                observedAt: observedAt,
                signalKey: signal.signalKey,
                valueNumber: nil,
                unit: signal.unit,
                status: .unavailable,
                confidence: nil,
                sourceSignal: signal.sourceSignal,
                rawPayloadRef: rawPayloadRef
            )
        } catch {
            // sendRawCommandWithTiming already captured the command-level error payload.
            return OBDSignalRecord(
                observedAt: observedAt,
                signalKey: signal.signalKey,
                valueNumber: nil,
                unit: signal.unit,
                status: .error,
                confidence: nil,
                sourceSignal: signal.sourceSignal,
                rawPayloadRef: buildErrorPayloadRef(command: signal.command, error: error)
            )
        }
    }

    /// When PID 0142 is unavailable, fall back to adapter-reported supply voltage (ATRV).
    /// This keeps MVP telemetry useful on vehicles/adapters that do not expose 0142.
    private func pollControlModuleVoltageFallback(
        signal: OBDStandardSignal,
        observedAt: Date,
        primaryResponse: String,
        primaryCommandStartedAt: Date,
        primaryCommandEndedAt: Date
    ) async -> OBDSignalRecord? {
        emitCommandExchange(
            startedAt: primaryCommandStartedAt,
            endedAt: primaryCommandEndedAt,
            command: signal.command,
            rawResponse: primaryResponse,
            errorMessage: nil,
            parseOutcome: .unavailable,
            signalKey: signal.signalKey,
            sourceSignal: signal.sourceSignal
        )

        do {
            let fallbackResult = try await sendRawCommandWithTiming(
                "ATRV",
                signalKey: signal.signalKey,
                sourceSignal: "AT_RV"
            )
            let fallbackResponse = fallbackResult.rawResponse
            guard let voltage = OBDResponseParser.decodeAdapterSupplyVoltage(
                rawResponse: fallbackResponse
            ) else {
                emitCommandExchange(
                    startedAt: fallbackResult.startedAt,
                    endedAt: fallbackResult.endedAt,
                    command: "ATRV",
                    rawResponse: fallbackResponse,
                    errorMessage: nil,
                    parseOutcome: .unavailable,
                    signalKey: signal.signalKey,
                    sourceSignal: "AT_RV"
                )
                return nil
            }

            emitCommandExchange(
                startedAt: fallbackResult.startedAt,
                endedAt: fallbackResult.endedAt,
                command: "ATRV",
                rawResponse: fallbackResponse,
                errorMessage: nil,
                parseOutcome: .ok,
                signalKey: signal.signalKey,
                sourceSignal: "AT_RV"
            )
            return OBDSignalRecord(
                observedAt: observedAt,
                signalKey: signal.signalKey,
                valueNumber: voltage,
                unit: signal.unit,
                status: .ok,
                confidence: min(signal.confidence, 0.75),
                sourceSignal: "AT_RV",
                rawPayloadRef: buildFallbackRawPayloadRef(
                    primaryCommand: signal.command,
                    primaryResponse: primaryResponse,
                    fallbackCommand: "ATRV",
                    fallbackResponse: fallbackResponse
                )
            )
        } catch {
            return nil
        }
    }

    func pollDiagnostics(observedAt: Date = Date()) async -> OBDDiagnosticSnapshot? {
        do {
            let readinessResult = try await sendRawCommandWithTiming(
                "0101",
                signalKey: "diag.readiness_summary",
                sourceSignal: "01_01"
            )
            let readinessResponse = readinessResult.rawResponse
            guard let readiness = OBDResponseParser.decodeReadinessStatus(rawResponse: readinessResponse) else {
                emitCommandExchange(
                    startedAt: readinessResult.startedAt,
                    endedAt: readinessResult.endedAt,
                    command: "0101",
                    rawResponse: readinessResponse,
                    errorMessage: nil,
                    parseOutcome: .unavailable,
                    signalKey: "diag.readiness_summary",
                    sourceSignal: "01_01"
                )
                return nil
            }
            emitCommandExchange(
                startedAt: readinessResult.startedAt,
                endedAt: readinessResult.endedAt,
                command: "0101",
                rawResponse: readinessResponse,
                errorMessage: nil,
                parseOutcome: .ok,
                signalKey: "diag.readiness_summary",
                sourceSignal: "01_01"
            )

            // Mode 03 can fail on some ECUs/adapters even when Mode 01 succeeds, so keep it best-effort.
            let dtcResult = try? await sendRawCommandWithTiming(
                "03",
                signalKey: "diag.dtcs_active",
                sourceSignal: "03"
            )
            if let dtcResult {
                emitCommandExchange(
                    startedAt: dtcResult.startedAt,
                    endedAt: dtcResult.endedAt,
                    command: "03",
                    rawResponse: dtcResult.rawResponse,
                    errorMessage: nil,
                    parseOutcome: .ok,
                    signalKey: "diag.dtcs_active",
                    sourceSignal: "03"
                )
            }
            let parsedCodes = dtcResult.map { response in
                OBDResponseParser.decodeStoredDiagnosticTroubleCodes(rawResponse: response.rawResponse)
            } ?? []
            let normalizedCodes: [String]
            if readiness.storedDTCCount > 0 {
                normalizedCodes = Array(parsedCodes.prefix(readiness.storedDTCCount))
            } else {
                normalizedCodes = []
            }

            return OBDDiagnosticSnapshot(
                observedAt: observedAt,
                milOn: readiness.milOn,
                dtcsActive: normalizedCodes
            )
        } catch {
            return nil
        }
    }

    private func probeAdapterIdentity() async throws -> OBDAdapterIdentity {
        let commandResult = try await sendRawCommandWithTiming("ATI")
        emitCommandExchange(
            startedAt: commandResult.startedAt,
            endedAt: commandResult.endedAt,
            command: "ATI",
            rawResponse: commandResult.rawResponse,
            errorMessage: nil,
            parseOutcome: .ok,
            signalKey: nil,
            sourceSignal: nil
        )
        return OBDAdapterIdentity.fromATIResponse(commandResult.rawResponse)
    }

    private func runInitializationPlan(for profile: OBDAdapterInitializationProfile) async throws {
        let steps = OBDInitializationPlan.steps(for: profile)
        for step in steps {
            do {
                let commandResult = try await sendRawCommandWithTiming(step.command)
                let rawResponse = commandResult.rawResponse
                if step.isRequired {
                    do {
                        try validateRequiredResponse(rawResponse, forCommand: step.command)
                    } catch {
                        emitCommandExchange(
                            startedAt: commandResult.startedAt,
                            endedAt: commandResult.endedAt,
                            command: step.command,
                            rawResponse: rawResponse,
                            errorMessage: OBDErrorPresentation.message(from: error),
                            parseOutcome: .error,
                            signalKey: nil,
                            sourceSignal: nil
                        )
                        throw error
                    }
                }
                emitCommandExchange(
                    startedAt: commandResult.startedAt,
                    endedAt: commandResult.endedAt,
                    command: step.command,
                    rawResponse: rawResponse,
                    errorMessage: nil,
                    parseOutcome: .ok,
                    signalKey: nil,
                    sourceSignal: nil
                )
            } catch {
                if step.isRequired {
                    throw error
                }
            }
        }
    }

    /// VeePeak/ELM adapters are more stable when we force CAN 11/500 first and then
    /// fall back to auto protocol if support probing still cannot confirm monitored PIDs.
    private func probeSupportedMode1PidsWithProtocolFallback(
        for signals: [OBDStandardSignal],
        profile: OBDAdapterInitializationProfile
    ) async -> Mode1SupportProbeResult {
        let protocolCommands = protocolFallbackCommands(for: profile)
        guard !protocolCommands.isEmpty else {
            return await probeSupportedMode1Pids(
                for: signals,
                attemptsPerBlock: supportProbeAttemptsPerBlock
            )
        }

        var summarySegments: [String] = []
        var lastProbeResult = Mode1SupportProbeResult(
            supportByBlock: [:],
            summarySegments: []
        )

        for protocolCommand in protocolCommands {
            do {
                let protocolResult = try await sendRawCommandWithTiming(protocolCommand)
                let protocolResponse = protocolResult.rawResponse
                emitCommandExchange(
                    startedAt: protocolResult.startedAt,
                    endedAt: protocolResult.endedAt,
                    command: protocolCommand,
                    rawResponse: protocolResponse,
                    errorMessage: nil,
                    parseOutcome: .ok,
                    signalKey: nil,
                    sourceSignal: nil
                )
                summarySegments.append(
                    "\(protocolCommand) set=\(normalizedSummarySegment(protocolResponse))"
                )
            } catch {
                summarySegments.append(
                    "\(protocolCommand) error=\(OBDErrorPresentation.message(from: error))"
                )
                continue
            }

            let probeResult = await probeSupportedMode1Pids(
                for: signals,
                attemptsPerBlock: supportProbeAttemptsPerBlock
            )
            let supportsMonitoredPids = containsMonitoredPidSupport(
                supportByBlock: probeResult.supportByBlock,
                signals: signals
            )

            let decoratedProbeSegments = probeResult.summarySegments.map { segment in
                "\(protocolCommand) \(segment)"
            }
            summarySegments.append(contentsOf: decoratedProbeSegments)

            if supportsMonitoredPids {
                return Mode1SupportProbeResult(
                    supportByBlock: probeResult.supportByBlock,
                    summarySegments: summarySegments
                )
            }

            summarySegments.append(
                "\(protocolCommand) probe=inconclusive_no_monitored_support"
            )
            lastProbeResult = Mode1SupportProbeResult(
                supportByBlock: [:],
                summarySegments: summarySegments
            )
        }

        if summarySegments.isEmpty {
            return await probeSupportedMode1Pids(
                for: signals,
                attemptsPerBlock: supportProbeAttemptsPerBlock
            )
        }
        return lastProbeResult
    }

    private func protocolFallbackCommands(
        for profile: OBDAdapterInitializationProfile
    ) -> [String] {
        switch profile {
        case .obdLink:
            return []
        case .elm327, .generic:
            return ["ATSP6", "ATSP0"]
        }
    }

    private func containsMonitoredPidSupport(
        supportByBlock: [UInt8: Set<UInt8>],
        signals: [OBDStandardSignal]
    ) -> Bool {
        let monitoredPids = Set(signals.map(\.mode1PID))
        return supportByBlock.values.contains { supportedPids in
            !supportedPids.intersection(monitoredPids).isEmpty
        }
    }

    /// Probe support bitmasks for only the PID blocks used by currently polled signals.
    private func probeSupportedMode1Pids(
        for signals: [OBDStandardSignal],
        attemptsPerBlock: Int
    ) async -> Mode1SupportProbeResult {
        let clampedAttempts = max(1, attemptsPerBlock)
        let blockBases = Array(Set(signals.map(\.supportBlockBasePID))).sorted()
        var supportByBlock: [UInt8: Set<UInt8>] = [:]
        var summarySegments: [String] = []

        for blockBase in blockBases {
            let command = mode1SupportCommand(for: blockBase)
            let monitoredPids = signals
                .filter { $0.supportBlockBasePID == blockBase }
                .map(\.mode1PID)
                .sorted()

            var didDecodeBlock = false
            for attempt in 1...clampedAttempts {
                do {
                    let commandResult = try await sendRawCommandWithTiming(command)
                    let rawResponse = commandResult.rawResponse
                    let normalizedResponse = normalizedSummarySegment(rawResponse)

                    guard let supportedPids = OBDResponseParser.decodeSupportedMode1Pids(
                        rawResponse: rawResponse,
                        blockBasePID: blockBase
                    ) else {
                        emitCommandExchange(
                            startedAt: commandResult.startedAt,
                            endedAt: commandResult.endedAt,
                            command: command,
                            rawResponse: rawResponse,
                            errorMessage: nil,
                            parseOutcome: .unavailable,
                            signalKey: nil,
                            sourceSignal: nil
                        )
                        summarySegments.append(
                            "\(command)#\(attempt) decode=none raw=\(normalizedResponse)"
                        )
                        if attempt < clampedAttempts {
                            try? await Task.sleep(nanoseconds: supportProbeRetryDelayNanoseconds)
                        }
                        continue
                    }

                    supportByBlock[blockBase] = supportedPids
                    let monitoredStatus = monitoredPids.map { pid in
                        let pidText = String(format: "%02X", pid)
                        let supportToken = supportedPids.contains(pid) ? "Y" : "N"
                        return "\(pidText):\(supportToken)"
                    }
                    .joined(separator: ",")

                    emitCommandExchange(
                        startedAt: commandResult.startedAt,
                        endedAt: commandResult.endedAt,
                        command: command,
                        rawResponse: rawResponse,
                        errorMessage: nil,
                        parseOutcome: .ok,
                        signalKey: nil,
                        sourceSignal: nil
                    )
                    summarySegments.append(
                        "\(command)#\(attempt) pids=[\(monitoredStatus)] raw=\(normalizedResponse)"
                    )
                    didDecodeBlock = true
                    break
                } catch {
                    summarySegments.append(
                        "\(command)#\(attempt) error=\(OBDErrorPresentation.message(from: error))"
                    )
                    if attempt < clampedAttempts {
                        try? await Task.sleep(nanoseconds: supportProbeRetryDelayNanoseconds)
                    }
                }
            }
            if !didDecodeBlock {
                continue
            }
        }

        return Mode1SupportProbeResult(
            supportByBlock: supportByBlock,
            summarySegments: summarySegments
        )
    }

    private func mode1SupportCommand(for blockBase: UInt8) -> String {
        String(format: "01%02X", blockBase)
    }

    private func probeActiveProtocolSummary() async -> String {
        do {
            let protocolResult = try await sendRawCommandWithTiming("ATDP")
            let protocolResponse = protocolResult.rawResponse
            emitCommandExchange(
                startedAt: protocolResult.startedAt,
                endedAt: protocolResult.endedAt,
                command: "ATDP",
                rawResponse: protocolResponse,
                errorMessage: nil,
                parseOutcome: .ok,
                signalKey: nil,
                sourceSignal: nil
            )
            return "ATDP raw=\(normalizedSummarySegment(protocolResponse))"
        } catch {
            return "ATDP error=\(OBDErrorPresentation.message(from: error))"
        }
    }

    private func buildBootstrapSessionEventPayloadRef(
        activeProfile: OBDAdapterInitializationProfile,
        supportProbeSummaries: [String]
    ) async -> String {
        let protocolSummary = await probeActiveProtocolSummary()
        let profileSummary = "profile=\(activeProfile.rawValue)"
        let summary = ([profileSummary, protocolSummary] + supportProbeSummaries).joined(separator: " | ")
        return truncateRawPayloadRef(summary)
    }

    private func isSignalExplicitlyUnsupported(_ signal: OBDStandardSignal) -> Bool {
        guard let supportedPids = supportedMode1PidsByBlock[signal.supportBlockBasePID] else {
            // Probe did not decode this block, so keep polling rather than suppressing data.
            return false
        }
        return !supportedPids.contains(signal.mode1PID)
    }

    private func sendRawCommandWithTiming(
        _ command: String,
        signalKey: String? = nil,
        sourceSignal: String? = nil
    ) async throws -> TimedCommandResult {
        let startedAt = Date()
        do {
            let rawResponse = try await transport.sendRawCommand(command)
            let endedAt = Date()
            return TimedCommandResult(
                startedAt: startedAt,
                endedAt: endedAt,
                rawResponse: rawResponse
            )
        } catch {
            let endedAt = Date()
            emitCommandExchange(
                startedAt: startedAt,
                endedAt: endedAt,
                command: command,
                rawResponse: nil,
                errorMessage: OBDErrorPresentation.message(from: error),
                parseOutcome: .error,
                signalKey: signalKey,
                sourceSignal: sourceSignal
            )
            throw error
        }
    }

    private func emitCommandExchange(
        startedAt: Date,
        endedAt: Date,
        command: String,
        rawResponse: String?,
        errorMessage: String?,
        parseOutcome: OBDCommandParseOutcome,
        signalKey: String?,
        sourceSignal: String?
    ) {
        guard let commandExchangeObserver else {
            return
        }

        commandExchangeObserver(
            OBDCommandExchange(
                startedAt: startedAt,
                endedAt: endedAt,
                command: command,
                rawResponse: rawResponse,
                errorMessage: errorMessage,
                parseOutcome: parseOutcome,
                signalKey: signalKey,
                sourceSignal: sourceSignal
            )
        )
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

    private func normalizedSummarySegment(_ rawResponse: String) -> String {
        let normalized = normalize(rawResponse)
        if normalized.isEmpty {
            return "<empty>"
        }
        return normalized
    }

    private func buildRawPayloadRef(command: String, rawResponse: String) -> String {
        // Keep a compact one-line command/response sample for downstream debug inspection.
        let normalizedResponse = normalize(rawResponse)
        return truncateRawPayloadRef("cmd=\(command) resp=\(normalizedResponse)")
    }

    private func buildFallbackRawPayloadRef(
        primaryCommand: String,
        primaryResponse: String,
        fallbackCommand: String,
        fallbackResponse: String
    ) -> String {
        let primarySummary = normalize(primaryResponse)
        let fallbackSummary = normalize(fallbackResponse)
        return truncateRawPayloadRef(
            "cmd=\(primaryCommand) resp=\(primarySummary) | fallback=\(fallbackCommand) resp=\(fallbackSummary)"
        )
    }

    private func buildUnsupportedPayloadRef(signal: OBDStandardSignal) -> String {
        let pid = String(format: "%02X", signal.mode1PID)
        let blockBase = String(format: "%02X", signal.supportBlockBasePID)
        return truncateRawPayloadRef(
            "cmd=\(signal.command) skipped=unsupported_pid (pid=0x\(pid) block=0x\(blockBase))"
        )
    }

    private func buildErrorPayloadRef(command: String, error: Error) -> String {
        truncateRawPayloadRef("cmd=\(command) error=\(OBDErrorPresentation.message(from: error))")
    }

    private func truncateRawPayloadRef(_ value: String) -> String {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.count > maxRawPayloadRefLength else {
            return trimmed
        }

        let endIndex = trimmed.index(trimmed.startIndex, offsetBy: maxRawPayloadRefLength - 3)
        return "\(trimmed[..<endIndex])..."
    }
}

private struct TimedCommandResult {
    let startedAt: Date
    let endedAt: Date
    let rawResponse: String
}

private struct Mode1SupportProbeResult {
    let supportByBlock: [UInt8: Set<UInt8>]
    let summarySegments: [String]
}
