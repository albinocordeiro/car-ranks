import Foundation
import Combine
#if canImport(UIKit)
import UIKit
#endif

@MainActor
final class OBDCaptureViewModel: ObservableObject {
    enum UploadState: Equatable {
        case idle
        case uploading
        case success(String)
        case error(String)
    }

    @Published private(set) var discoveredDevices: [OBDAdapterDevice] = []
    @Published private(set) var connectionState: OBDConnectionState = .disconnected
    @Published private(set) var isCapturing = false
    @Published private(set) var isCaptureStarting = false
    @Published private(set) var captureStatusMessage = "Connect and initialize an adapter to start capture."
    @Published private(set) var pendingRecordCount = 0
    @Published private(set) var pendingDiagnosticCount = 0
    @Published private(set) var pendingSessionEventCount = 0
    @Published private(set) var pendingUploadSummary: OBDPendingUploadSummary = .empty
    @Published private(set) var recentRecords: [OBDSignalRecord] = []
    @Published private(set) var statusMessage = "Connect an adapter to start capture."
    @Published private(set) var adapterIdentitySummary = "Unknown"
    @Published private(set) var initializationProfileSummary = "Not initialized"
    @Published private(set) var diagnosticPresentation = OBDDiagnosticPresentation.from(
        latestSnapshot: nil,
        lastChangedAt: nil
    )
    @Published private(set) var queuedBatchCount = 0
    @Published private(set) var uploadState: UploadState = .idle
    @Published private(set) var lastQueuedBatchSummary = "No batches queued yet."
    @Published private(set) var lastSuccessfulUploadSummary = "No successful uploads yet."
    @Published private(set) var lastUploadIdentifiersSummary = "No successful upload IDs yet."
    @Published private(set) var uploadRetryCountdownText: String?
    @Published private(set) var pendingBatchPreview = "Generate preview to inspect pending payload."
    @Published private(set) var lastRunPackExportSummary = "No run pack exported yet."
    @Published private(set) var runPackShareURL: URL?
    @Published var sampleIntervalSecondsText = "5"

    private let bleClient: CoreBluetoothOBDClient
    private let captureCoordinator: OBDTelemetryCaptureCoordinator
    private let uploadQueueCoordinator: TelemetryUploadQueueCoordinator
    private let runPackStore: OBDCaptureRunPackStore
    private let sessionProvider: () -> SessionContext
    private let appVersionProvider: () -> String
    private var lastUploadReceipt: OBDRunPackUploadReceipt?
    private var shouldResumeCaptureAfterReconnect = false
    private var cancellables: Set<AnyCancellable> = []

    init(
        bleClient: CoreBluetoothOBDClient,
        captureCoordinator: OBDTelemetryCaptureCoordinator,
        uploadQueueCoordinator: TelemetryUploadQueueCoordinator,
        runPackStore: OBDCaptureRunPackStore = OBDCaptureRunPackStore(),
        sessionProvider: @escaping () -> SessionContext,
        appVersionProvider: @escaping () -> String
    ) {
        self.bleClient = bleClient
        self.captureCoordinator = captureCoordinator
        self.uploadQueueCoordinator = uploadQueueCoordinator
        self.runPackStore = runPackStore
        self.sessionProvider = sessionProvider
        self.appVersionProvider = appVersionProvider
        bind()
    }

    func startScanning() {
        bleClient.startScanning()
        if case .scanning = bleClient.connectionState {
            statusMessage = "Scanning for OBD adapters..."
        }
    }

    func stopScanning() {
        bleClient.stopScanning()
        statusMessage = "Scan stopped."
    }

    func connect(to deviceID: UUID) {
        bleClient.connect(to: deviceID)
        uploadState = .idle
    }

    func disconnect() {
        shouldResumeCaptureAfterReconnect = false
        if isCapturing {
            captureCoordinator.stopCapture()
        }
        bleClient.disconnect()
        uploadState = .idle
        statusMessage = "Adapter disconnected."
        captureStatusMessage = "Adapter disconnected."
    }

    func toggleCapture() {
        if isCapturing {
            shouldResumeCaptureAfterReconnect = false
            captureCoordinator.stopCapture()
            statusMessage = "Capture stopped. Pending payload: \(pendingUploadSummary.inlineDescription)."
            captureStatusMessage = "Capture stopped. Pending payload: \(pendingUploadSummary.inlineDescription)."
            return
        }

        guard connectionState.isConnected else {
            statusMessage = "Connect an adapter before starting capture."
            captureStatusMessage = "Connect an adapter before starting capture."
            return
        }

        let interval = parsedSampleInterval
        sampleIntervalSecondsText = String(interval)
        captureCoordinator.startCapture(sampleIntervalSeconds: interval)
        uploadState = .idle
        statusMessage = "Starting capture with \(interval)s interval..."
        captureStatusMessage = "Initializing adapter..."
    }

    func uploadPendingBatch() {
        guard !isCapturing else {
            statusMessage = "Stop capture before uploading a batch."
            return
        }

        let session = sessionProvider()
        guard let batchBundle = captureCoordinator.buildBatch(
            vehicleUID: session.vehicleUID,
            appVersion: appVersionProvider(),
            adapterFingerprint: bleClient.adapterFingerprint
        ) else {
            statusMessage = "No captured records to upload."
            return
        }

        uploadQueueCoordinator.enqueue(
            batch: batchBundle.request,
            captureWindowEndedAt: batchBundle.windowEndedAt
        )
        captureCoordinator.clearPendingData(afterWindowEndedAt: batchBundle.windowEndedAt)
        let queuedSummary = OBDPendingUploadSummary.from(batch: batchBundle.request)
        uploadState = .success("Queued telemetry batch (\(queuedSummary.inlineDescription)).")
        statusMessage = "Queued telemetry batch (\(queuedSummary.inlineDescription)). Upload will retry automatically."
        pendingBatchPreview = "Generate preview to inspect pending payload."
    }

    func retryQueuedUploads() {
        uploadQueueCoordinator.triggerUpload()
        statusMessage = "Retrying queued uploads..."
    }

    var retryQueuedUploadsButtonTitle: String {
        guard let uploadRetryCountdownText else {
            return "Retry Queued Uploads"
        }
        return "Retry Queued Uploads (\(uploadRetryCountdownText))"
    }

    var isRetryQueuedUploadsDisabled: Bool {
        uploadRetryCountdownText != nil
    }

    var canCopyLastUploadIDs: Bool {
        lastUploadReceipt != nil
    }

    func refreshPendingBatchPreview() {
        let session = sessionProvider()
        guard let batchBundle = captureCoordinator.buildBatch(
            vehicleUID: session.vehicleUID,
            appVersion: appVersionProvider(),
            adapterFingerprint: bleClient.adapterFingerprint
        ) else {
            pendingBatchPreview = "No pending payload to preview."
            statusMessage = "No captured payload available for preview."
            return
        }

        do {
            pendingBatchPreview = try TelemetryBatchPreviewFormatter.prettyPrintedJSON(from: batchBundle.request)
            statusMessage = "Generated pending payload preview."
        } catch {
            pendingBatchPreview = "Failed to encode pending payload: \(error.localizedDescription)"
            statusMessage = "Pending payload preview failed."
        }
    }

    func exportLastRunPack() {
        guard let runPack = buildLastRunPackForExport() else {
            lastRunPackExportSummary = "No completed run available. Stop a capture first."
            statusMessage = "No completed run available. Stop a capture first."
            return
        }

        do {
            let url = try runPackStore.save(runPack)
            runPackShareURL = url
            lastRunPackExportSummary = "Exported \(url.lastPathComponent)"
            statusMessage = "Exported run pack \(url.lastPathComponent)."
        } catch {
            runPackShareURL = nil
            lastRunPackExportSummary = "Run pack export failed: \(error.localizedDescription)"
            statusMessage = "Run pack export failed: \(error.localizedDescription)"
        }
    }

    func prepareLastRunPackForShare() {
        guard runPackShareURL == nil else {
            return
        }
        exportLastRunPack()
    }

    func copyLastUploadIDs() {
        guard let lastUploadReceipt else {
            statusMessage = "No successful upload IDs available to copy."
            return
        }

        let payload = Self.uploadIdentifierPayload(from: lastUploadReceipt)
        #if canImport(UIKit)
        UIPasteboard.general.string = payload
        statusMessage = "Copied last upload IDs."
        #else
        statusMessage = "Clipboard copy is unavailable on this platform."
        #endif
    }

    private var parsedSampleInterval: Int {
        guard let value = Int(sampleIntervalSecondsText.trimmingCharacters(in: .whitespacesAndNewlines)),
              value > 0
        else {
            return 5
        }
        return min(value, 300)
    }

    private func bind() {
        bleClient.$discoveredDevices
            .sink { [weak self] in
                self?.discoveredDevices = $0
            }
            .store(in: &cancellables)

        bleClient.$connectionState
            .sink { [weak self] state in
                guard let self else { return }
                connectionState = state

                switch state {
                case .connected:
                    if shouldResumeCaptureAfterReconnect {
                        shouldResumeCaptureAfterReconnect = false
                        let interval = parsedSampleInterval
                        captureCoordinator.startCapture(sampleIntervalSeconds: interval)
                        statusMessage = "Adapter reconnected. Capture resumed every \(interval)s."
                        captureStatusMessage = "Adapter reconnected. Resuming capture..."
                    }
                case .reconnecting:
                    if isCapturing {
                        shouldResumeCaptureAfterReconnect = true
                        captureCoordinator.stopCapture()
                        statusMessage = "Adapter link dropped. Waiting to reconnect..."
                        captureStatusMessage = "Adapter link dropped. Waiting to reconnect..."
                    }
                case .error:
                    shouldResumeCaptureAfterReconnect = false
                    if isCapturing {
                        captureCoordinator.stopCapture()
                    }
                    if case let .error(message) = state {
                        statusMessage = message
                        captureStatusMessage = message
                    }
                case .disconnected:
                    shouldResumeCaptureAfterReconnect = false
                    if isCapturing {
                        captureCoordinator.stopCapture()
                        statusMessage = "Capture stopped because adapter disconnected."
                        captureStatusMessage = "Capture stopped because adapter disconnected."
                    }
                case .scanning, .connecting:
                    break
                }
            }
            .store(in: &cancellables)

        captureCoordinator.$isCapturing
            .sink { [weak self] isCapturing in
                guard let self else { return }
                self.isCapturing = isCapturing
                if isCapturing && !self.isCaptureStarting {
                    self.captureStatusMessage = "Capture running."
                }
            }
            .store(in: &cancellables)

        captureCoordinator.$isBootstrapping
            .sink { [weak self] isBootstrapping in
                guard let self else { return }
                self.isCaptureStarting = isBootstrapping
                if isBootstrapping {
                    self.captureStatusMessage = "Initializing adapter..."
                } else if self.isCapturing {
                    self.captureStatusMessage = "Capture running."
                }
            }
            .store(in: &cancellables)

        captureCoordinator.$pendingRecordCount
            .sink { [weak self] in
                self?.pendingRecordCount = $0
            }
            .store(in: &cancellables)

        captureCoordinator.$pendingDiagnosticCount
            .sink { [weak self] in
                self?.pendingDiagnosticCount = $0
            }
            .store(in: &cancellables)

        captureCoordinator.$pendingSessionEventCount
            .sink { [weak self] in
                self?.pendingSessionEventCount = $0
            }
            .store(in: &cancellables)

        captureCoordinator.$recentRecords
            .sink { [weak self] in
                self?.recentRecords = Array($0.prefix(40))
            }
            .store(in: &cancellables)

        captureCoordinator.$lastCaptureError
            .compactMap { $0 }
            .sink { [weak self] in
                self?.statusMessage = $0
                self?.captureStatusMessage = $0
            }
            .store(in: &cancellables)

        captureCoordinator.$adapterIdentitySummary
            .sink { [weak self] summary in
                self?.adapterIdentitySummary = summary ?? "Unknown"
            }
            .store(in: &cancellables)

        captureCoordinator.$initializationProfileSummary
            .sink { [weak self] summary in
                self?.initializationProfileSummary = summary ?? "Not initialized"
            }
            .store(in: &cancellables)

        Publishers.CombineLatest(
            captureCoordinator.$latestDiagnosticSnapshot,
            captureCoordinator.$lastDiagnosticStateChangedAt
        )
        .sink { [weak self] latestSnapshot, lastChangedAt in
            self?.diagnosticPresentation = OBDDiagnosticPresentation.from(
                latestSnapshot: latestSnapshot,
                lastChangedAt: lastChangedAt
            )
        }
        .store(in: &cancellables)

        Publishers.CombineLatest3(
            captureCoordinator.$pendingRecordCount,
            captureCoordinator.$pendingDiagnosticCount,
            captureCoordinator.$pendingSessionEventCount
        )
        .sink { [weak self] recordCount, diagnosticCount, sessionEventCount in
            self?.pendingUploadSummary = OBDPendingUploadSummary(
                signalRecordCount: recordCount,
                diagnosticEventCount: diagnosticCount,
                sessionEventCount: sessionEventCount
            )
        }
        .store(in: &cancellables)

        uploadQueueCoordinator.$pendingBatchCount
            .sink { [weak self] count in
                self?.queuedBatchCount = count
            }
            .store(in: &cancellables)

        uploadQueueCoordinator.$lastQueuedBatchSummary
            .sink { [weak self] summary in
                self?.lastQueuedBatchSummary = summary ?? "No batches queued yet."
            }
            .store(in: &cancellables)

        uploadQueueCoordinator.$lastSuccessfulUploadSummary
            .sink { [weak self] summary in
                self?.lastSuccessfulUploadSummary = summary ?? "No successful uploads yet."
            }
            .store(in: &cancellables)

        Publishers.CombineLatest(
            uploadQueueCoordinator.$lastSuccessfulUploadResponse,
            uploadQueueCoordinator.$lastSuccessfulUploadAt
        )
        .sink { [weak self] response, uploadedAt in
            guard let self else { return }
            guard let response, let uploadedAt else {
                return
            }

            let receipt = OBDRunPackUploadReceipt(
                batchID: response.batchID,
                ingestID: response.ingestID,
                accepted: response.accepted,
                uploadedAt: uploadedAt,
                message: response.duplicate ? "duplicate batch" : nil
            )
            lastUploadReceipt = receipt
            lastUploadIdentifiersSummary = Self.uploadIdentifierPayload(from: receipt)
        }
        .store(in: &cancellables)

        uploadQueueCoordinator.$isUploading
            .sink { [weak self] isUploading in
                guard let self else { return }
                if isUploading {
                    uploadState = .uploading
                } else if case .uploading = uploadState {
                    uploadState = .idle
                }
            }
            .store(in: &cancellables)

        uploadQueueCoordinator.$lastUploadMessage
            .compactMap { $0 }
            .sink { [weak self] message in
                guard let self else { return }
                statusMessage = message
                if message.hasPrefix("Dropped queued telemetry batch") {
                    uploadState = .error(message)
                } else if message.hasPrefix("Uploaded queued telemetry batch") {
                    uploadState = .success(message)
                }
            }
            .store(in: &cancellables)

        uploadQueueCoordinator.$nextRetryInSeconds
            .sink { [weak self] nextRetryInSeconds in
                guard let self else { return }
                guard let nextRetryInSeconds, nextRetryInSeconds > 0 else {
                    uploadRetryCountdownText = nil
                    return
                }
                uploadRetryCountdownText = Self.formatRetryCountdown(nextRetryInSeconds)
            }
            .store(in: &cancellables)

        captureCoordinator.$lastCompletedRunSessionID
            .sink { [weak self] sessionID in
                guard let self else { return }
                guard let sessionID else {
                    return
                }
                if runPackShareURL == nil {
                    lastRunPackExportSummary = "Run \(sessionID.uuidString.lowercased()) ready to export."
                }
            }
            .store(in: &cancellables)
    }

    private static func formatRetryCountdown(_ totalSeconds: Int) -> String {
        let clampedSeconds = max(0, totalSeconds)
        let hours = clampedSeconds / 3_600
        let minutes = (clampedSeconds % 3_600) / 60
        let seconds = clampedSeconds % 60

        if hours > 0 {
            return String(format: "%d:%02d:%02d", hours, minutes, seconds)
        }
        return String(format: "%02d:%02d", minutes, seconds)
    }

    private func buildLastRunPackForExport() -> OBDCaptureRunPack? {
        let sessionContext = sessionProvider()
        return captureCoordinator.buildLastRunPack(
            sessionContext: sessionContext,
            appVersion: appVersionProvider(),
            adapterFingerprint: bleClient.adapterFingerprint,
            uploadReceipt: lastUploadReceipt
        )
    }

    private static func uploadIdentifierPayload(from receipt: OBDRunPackUploadReceipt) -> String {
        "batch_id=\(receipt.batchID.uuidString.lowercased()) ingest_id=\(receipt.ingestID.uuidString.lowercased())"
    }
}
