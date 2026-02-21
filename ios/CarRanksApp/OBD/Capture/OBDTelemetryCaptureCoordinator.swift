import Foundation

/// Owns the capture loop and batch window boundaries.
@MainActor
final class OBDTelemetryCaptureCoordinator: ObservableObject {
    @Published private(set) var isCapturing = false
    @Published private(set) var recentRecords: [OBDSignalRecord] = []
    @Published private(set) var pendingRecordCount = 0
    @Published private(set) var lastCaptureError: String?
    @Published private(set) var adapterIdentitySummary: String?
    @Published private(set) var initializationProfileSummary: String?

    private let commandExecutor: OBDCommandExecutor
    private let now: () -> Date

    private var captureTask: Task<Void, Never>?
    private var captureWindowStartedAt: Date?
    private var currentSampleIntervalSeconds = 5
    private var pendingRecords: [OBDSignalRecord] = []
    private var pendingSessionEvents: [TelemetryBatchRequest.SessionEvent] = []
    private var activeSessionID: UUID?

    init(
        commandExecutor: OBDCommandExecutor,
        now: @escaping () -> Date = Date.init
    ) {
        self.commandExecutor = commandExecutor
        self.now = now
    }

    func startCapture(sampleIntervalSeconds: Int) {
        guard !isCapturing else { return }

        currentSampleIntervalSeconds = max(1, sampleIntervalSeconds)
        let startedAt = now()
        captureWindowStartedAt = startedAt
        activeSessionID = UUID()
        if let activeSessionID {
            pendingSessionEvents.append(
                TelemetryBatchRequest.SessionEvent(
                    eventType: "drive_session_start",
                    observedAt: TelemetryTimestampFormatter.string(from: startedAt),
                    sessionID: activeSessionID
                )
            )
        }

        isCapturing = true
        lastCaptureError = nil

        captureTask = Task { [weak self] in
            await self?.runCaptureLoop()
        }
    }

    func stopCapture() {
        guard isCapturing else { return }

        isCapturing = false
        captureTask?.cancel()
        captureTask = nil
        commandExecutor.resetBootstrapState()

        let stoppedAt = now()
        if let activeSessionID {
            pendingSessionEvents.append(
                TelemetryBatchRequest.SessionEvent(
                    eventType: "drive_session_stop",
                    observedAt: TelemetryTimestampFormatter.string(from: stoppedAt),
                    sessionID: activeSessionID
                )
            )
        }
        activeSessionID = nil
    }

    func buildBatch(
        vehicleUID: UUID,
        appVersion: String,
        adapterFingerprint: String?
    ) -> (request: TelemetryBatchRequest, windowEndedAt: Date)? {
        guard let captureWindowStartedAt,
              !pendingRecords.isEmpty || !pendingSessionEvents.isEmpty
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
                sampleIntervalSeconds: currentSampleIntervalSeconds
            ),
            records: pendingRecords.map(TelemetryBatchRequest.SignalRecord.from),
            sessionEvents: pendingSessionEvents,
            diagnostics: []
        )

        return (batch, endedAt)
    }

    func clearPendingData(afterWindowEndedAt endedAt: Date) {
        pendingRecords.removeAll()
        pendingSessionEvents.removeAll()
        pendingRecordCount = 0
        captureWindowStartedAt = endedAt
    }

    private func runCaptureLoop() async {
        do {
            try await commandExecutor.bootstrapAdapterIfNeeded()
            initializationProfileSummary = commandExecutor.activeInitializationProfile?.rawValue.uppercased()
            adapterIdentitySummary = commandExecutor.detectedIdentity?.normalizedValue ?? "Unknown adapter identity"
        } catch {
            lastCaptureError = error.localizedDescription
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
                append(record: record)
            }

            let sleepNanoseconds = UInt64(currentSampleIntervalSeconds) * 1_000_000_000
            try? await Task.sleep(nanoseconds: sleepNanoseconds)
        }
    }

    private func append(record: OBDSignalRecord) {
        pendingRecords.append(record)
        pendingRecordCount = pendingRecords.count

        // Keep UI rendering bounded while preserving enough history for visual troubleshooting.
        recentRecords.insert(record, at: 0)
        if recentRecords.count > 150 {
            recentRecords.removeLast(recentRecords.count - 150)
        }

        if record.status == .error {
            lastCaptureError = "Last adapter read failed for \(record.signalKey)."
        }
    }
}
