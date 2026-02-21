import Foundation
import Combine

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
    @Published var sampleIntervalSecondsText = "5"

    private let bleClient: CoreBluetoothOBDClient
    private let captureCoordinator: OBDTelemetryCaptureCoordinator
    private let uploadQueueCoordinator: TelemetryUploadQueueCoordinator
    private let sessionProvider: () -> SessionContext
    private let appVersionProvider: () -> String
    private var shouldResumeCaptureAfterReconnect = false
    private var cancellables: Set<AnyCancellable> = []

    init(
        bleClient: CoreBluetoothOBDClient,
        captureCoordinator: OBDTelemetryCaptureCoordinator,
        uploadQueueCoordinator: TelemetryUploadQueueCoordinator,
        sessionProvider: @escaping () -> SessionContext,
        appVersionProvider: @escaping () -> String
    ) {
        self.bleClient = bleClient
        self.captureCoordinator = captureCoordinator
        self.uploadQueueCoordinator = uploadQueueCoordinator
        self.sessionProvider = sessionProvider
        self.appVersionProvider = appVersionProvider
        bind()
    }

    func startScanning() {
        bleClient.startScanning()
        statusMessage = "Scanning for OBD adapters..."
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
    }

    func toggleCapture() {
        if isCapturing {
            shouldResumeCaptureAfterReconnect = false
            captureCoordinator.stopCapture()
            statusMessage = "Capture stopped. Pending payload: \(pendingUploadSummary.inlineDescription)."
            return
        }

        guard connectionState.isConnected else {
            statusMessage = "Connect an adapter before starting capture."
            return
        }

        let interval = parsedSampleInterval
        sampleIntervalSecondsText = String(interval)
        captureCoordinator.startCapture(sampleIntervalSeconds: interval)
        uploadState = .idle
        statusMessage = "Capturing signals every \(interval)s."
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
    }

    func retryQueuedUploads() {
        uploadQueueCoordinator.triggerUpload()
        statusMessage = "Retrying queued uploads..."
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
                    }
                case .reconnecting:
                    if isCapturing {
                        shouldResumeCaptureAfterReconnect = true
                        captureCoordinator.stopCapture()
                        statusMessage = "Adapter link dropped. Waiting to reconnect..."
                    }
                case .error:
                    shouldResumeCaptureAfterReconnect = false
                    if isCapturing {
                        captureCoordinator.stopCapture()
                    }
                    if case let .error(message) = state {
                        statusMessage = message
                    }
                case .disconnected:
                    shouldResumeCaptureAfterReconnect = false
                    if isCapturing {
                        captureCoordinator.stopCapture()
                        statusMessage = "Capture stopped because adapter disconnected."
                    }
                case .scanning, .connecting:
                    break
                }
            }
            .store(in: &cancellables)

        captureCoordinator.$isCapturing
            .sink { [weak self] in
                self?.isCapturing = $0
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
    }
}
