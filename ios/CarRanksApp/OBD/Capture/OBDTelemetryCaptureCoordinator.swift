import Foundation

/// Owns the capture loop and batch window boundaries.
@MainActor
final class OBDTelemetryCaptureCoordinator: ObservableObject {
    @Published private(set) var isCapturing = false
    @Published private(set) var isBootstrapping = false
    @Published private(set) var recentRecords: [OBDSignalRecord] = []
    @Published private(set) var pendingRecordCount = 0
    @Published private(set) var pendingDiagnosticCount = 0
    @Published private(set) var pendingSessionEventCount = 0
    @Published private(set) var lastCaptureError: String?
    @Published private(set) var adapterIdentitySummary: String?
    @Published private(set) var initializationProfileSummary: String?
    @Published private(set) var latestDiagnosticSnapshot: OBDDiagnosticSnapshot?
    @Published private(set) var lastDiagnosticStateChangedAt: Date?
    @Published private(set) var lastCompletedRunSessionID: UUID?
    @Published private(set) var commandExchangeCount = 0

    private let commandExecutor: OBDCommandExecutor
    private let now: () -> Date

    private var captureTask: Task<Void, Never>?
    private var captureWindowStartedAt: Date?
    private var currentSampleIntervalSeconds = 5
    private var pendingRecords: [OBDSignalRecord] = []
    private var pendingDiagnostics: [OBDDiagnosticSnapshot] = []
    private var pendingSessionEvents: [TelemetryBatchRequest.SessionEvent] = []
    private var activeSessionID: UUID?
    private var commandExchanges: [OBDCommandExchange] = []
    private var lastCompletedRunSnapshot: CompletedRunSnapshot?
    private var lastDiagnosticPollAt: Date?
    private var lastDiagnosticStateSignature: String?

    private let diagnosticPollIntervalSeconds = 30.0
    private let minimumUploadIntervalSeconds = 60
    private let maxSessionEventRawPayloadRefLength = 500

    init(
        commandExecutor: OBDCommandExecutor,
        now: @escaping () -> Date = Date.init
    ) {
        self.commandExecutor = commandExecutor
        self.now = now
        commandExecutor.setCommandExchangeObserver { [weak self] exchange in
            self?.appendCommandExchange(exchange)
        }
    }

    func startCapture(sampleIntervalSeconds: Int) {
        guard !isCapturing else { return }

        currentSampleIntervalSeconds = max(1, sampleIntervalSeconds)
        if captureWindowStartedAt == nil {
            captureWindowStartedAt = now()
        }
        latestDiagnosticSnapshot = nil
        lastDiagnosticStateChangedAt = nil
        lastDiagnosticPollAt = nil
        lastDiagnosticStateSignature = nil
        activeSessionID = nil
        commandExchanges.removeAll()
        commandExchangeCount = 0

        isCapturing = true
        isBootstrapping = true
        lastCaptureError = nil

        captureTask = Task { [weak self] in
            await self?.runCaptureLoop()
        }
    }

    func stopCapture() {
        guard isCapturing else { return }

        isCapturing = false
        isBootstrapping = false
        captureTask?.cancel()
        captureTask = nil
        commandExecutor.resetBootstrapState()

        let stoppedAt = now()
        let finalizedSessionID = activeSessionID
        if let activeSessionID {
            pendingSessionEvents.append(
                TelemetryBatchRequest.SessionEvent(
                    eventType: "drive_session_stop",
                    observedAt: TelemetryTimestampFormatter.string(from: stoppedAt),
                    sessionID: activeSessionID
                )
            )
        }
        if let finalizedSessionID,
           let captureWindowStartedAt
        {
            lastCompletedRunSnapshot = CompletedRunSnapshot(
                sessionID: finalizedSessionID,
                captureWindowStartedAt: captureWindowStartedAt,
                captureWindowEndedAt: stoppedAt,
                sampleIntervalSeconds: currentSampleIntervalSeconds,
                adapterIdentitySummary: adapterIdentitySummary,
                initializationProfileSummary: initializationProfileSummary,
                commandExchanges: commandExchanges
            )
            lastCompletedRunSessionID = finalizedSessionID
        }
        activeSessionID = nil
        refreshPendingCounts()
    }

    func buildBatch(
        vehicleUID: UUID,
        appVersion: String,
        adapterFingerprint: String?
    ) -> (request: TelemetryBatchRequest, windowEndedAt: Date)? {
        guard let captureWindowStartedAt,
              !pendingRecords.isEmpty || !pendingSessionEvents.isEmpty || !pendingDiagnostics.isEmpty
        else {
            return nil
        }

        let endedAt = now()
        let fallbackFingerprint = "unknown-adapter"
        let batch = TelemetryBatchRequest(
            batchID: UUID(),
            vehicleUID: vehicleUID,
            client: .init(
                appVersion: appVersion,
                adapterFingerprint: adapterFingerprint ?? fallbackFingerprint
            ),
            captureWindow: .init(
                startedAt: TelemetryTimestampFormatter.string(from: captureWindowStartedAt),
                endedAt: TelemetryTimestampFormatter.string(from: endedAt),
                // Backend currently validates this field against upload cadence bounds (>= 60s).
                sampleIntervalSeconds: max(minimumUploadIntervalSeconds, currentSampleIntervalSeconds)
            ),
            records: pendingRecords.map(TelemetryBatchRequest.SignalRecord.from),
            sessionEvents: pendingSessionEvents,
            diagnostics: pendingDiagnostics.map(TelemetryBatchRequest.DiagnosticEvent.from)
        )

        return (batch, endedAt)
    }

    func clearPendingData(afterWindowEndedAt endedAt: Date) {
        pendingRecords.removeAll()
        pendingDiagnostics.removeAll()
        pendingSessionEvents.removeAll()
        refreshPendingCounts()
        captureWindowStartedAt = endedAt
    }

    func buildLastRunPack(
        sessionContext: SessionContext,
        appVersion: String,
        adapterFingerprint: String?,
        uploadReceipt: OBDRunPackUploadReceipt?
    ) -> OBDCaptureRunPack? {
        guard let snapshot = lastCompletedRunSnapshot else {
            return nil
        }

        return OBDCaptureRunPack(
            sessionID: snapshot.sessionID,
            userID: sessionContext.userID,
            vehicleUID: sessionContext.vehicleUID,
            appVersion: appVersion,
            adapterFingerprint: adapterFingerprint,
            adapterIdentitySummary: snapshot.adapterIdentitySummary,
            initializationProfileSummary: snapshot.initializationProfileSummary,
            captureWindowStartedAt: snapshot.captureWindowStartedAt,
            captureWindowEndedAt: snapshot.captureWindowEndedAt,
            sampleIntervalSeconds: snapshot.sampleIntervalSeconds,
            commandExchanges: snapshot.commandExchanges,
            uploadReceipt: uploadReceipt
        )
    }

    private func runCaptureLoop() async {
        defer {
            captureTask = nil
        }

        do {
            try await commandExecutor.bootstrapAdapterIfNeeded()
            guard isCapturing, !Task.isCancelled else {
                return
            }
            isBootstrapping = false
            initializationProfileSummary = commandExecutor.activeInitializationProfile?.rawValue.uppercased()
            adapterIdentitySummary = commandExecutor.detectedIdentity?.normalizedValue ?? "Unknown adapter identity"
            beginDriveSession(
                rawPayloadRef: commandExecutor.bootstrapSessionEventPayloadRef
            )
        } catch {
            isBootstrapping = false
            lastCaptureError = "Capture failed to start: \(OBDErrorPresentation.message(from: error))"
            activeSessionID = nil
            if pendingRecords.isEmpty && pendingSessionEvents.isEmpty && pendingDiagnostics.isEmpty {
                captureWindowStartedAt = nil
            }
            isCapturing = false
            return
        }

        while !Task.isCancelled, isCapturing {
            let observedAt = now()
            for signal in OBDStandardSignal.allCases {
                if Task.isCancelled || !isCapturing {
                    break
                }

                let record = await commandExecutor.poll(signal: signal, observedAt: observedAt)
                append(record: record.withSessionID(activeSessionID))
            }

            if shouldPollDiagnostics(at: observedAt),
               let diagnosticSnapshot = await commandExecutor.pollDiagnostics(observedAt: observedAt)
            {
                appendDiagnostic(snapshot: diagnosticSnapshot)
            }

            let sleepNanoseconds = UInt64(currentSampleIntervalSeconds) * 1_000_000_000
            try? await Task.sleep(nanoseconds: sleepNanoseconds)
        }
    }

    private func beginDriveSession(rawPayloadRef: String?) {
        let startedAt = now()
        activeSessionID = UUID()
        if let activeSessionID {
            pendingSessionEvents.append(
                TelemetryBatchRequest.SessionEvent(
                    eventType: "drive_session_start",
                    observedAt: TelemetryTimestampFormatter.string(from: startedAt),
                    sessionID: activeSessionID,
                    rawPayloadRef: truncateSessionEventRawPayloadRef(rawPayloadRef)
                )
            )
            refreshPendingCounts()
        }
    }

    private func truncateSessionEventRawPayloadRef(_ value: String?) -> String? {
        guard let value else {
            return nil
        }

        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            return nil
        }
        guard trimmed.count > maxSessionEventRawPayloadRefLength else {
            return trimmed
        }

        let endIndex = trimmed.index(
            trimmed.startIndex,
            offsetBy: maxSessionEventRawPayloadRefLength - 3
        )
        return "\(trimmed[..<endIndex])..."
    }

    private func append(record: OBDSignalRecord) {
        pendingRecords.append(record)
        refreshPendingCounts()

        // Keep UI rendering bounded while preserving enough history for visual troubleshooting.
        recentRecords.insert(record, at: 0)
        if recentRecords.count > 150 {
            recentRecords.removeLast(recentRecords.count - 150)
        }

        if record.status == .error {
            lastCaptureError = "Last adapter read failed for \(record.signalKey)."
        }
    }

    private func shouldPollDiagnostics(at observedAt: Date) -> Bool {
        guard let lastDiagnosticPollAt else {
            return true
        }
        return observedAt.timeIntervalSince(lastDiagnosticPollAt) >= diagnosticPollIntervalSeconds
    }

    private func appendDiagnostic(snapshot: OBDDiagnosticSnapshot) {
        lastDiagnosticPollAt = snapshot.observedAt
        latestDiagnosticSnapshot = snapshot

        // Keep only state changes so telemetry remains compact but still debuggable.
        guard snapshot.stateSignature != lastDiagnosticStateSignature else {
            return
        }

        lastDiagnosticStateSignature = snapshot.stateSignature
        lastDiagnosticStateChangedAt = snapshot.observedAt
        pendingDiagnostics.append(snapshot)
        refreshPendingCounts()
    }

    private func appendCommandExchange(_ exchange: OBDCommandExchange) {
        commandExchanges.append(exchange)
        commandExchangeCount = commandExchanges.count
    }

    private func refreshPendingCounts() {
        pendingRecordCount = pendingRecords.count
        pendingDiagnosticCount = pendingDiagnostics.count
        pendingSessionEventCount = pendingSessionEvents.count
    }
}

private struct CompletedRunSnapshot {
    let sessionID: UUID
    let captureWindowStartedAt: Date
    let captureWindowEndedAt: Date
    let sampleIntervalSeconds: Int
    let adapterIdentitySummary: String?
    let initializationProfileSummary: String?
    let commandExchanges: [OBDCommandExchange]
}
