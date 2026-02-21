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
    @Published private(set) var recentRecords: [OBDSignalRecord] = []
    @Published private(set) var statusMessage = "Connect an adapter to start capture."
    @Published private(set) var uploadState: UploadState = .idle
    @Published var sampleIntervalSecondsText = "5"

    private let bleClient: CoreBluetoothOBDClient
    private let captureCoordinator: OBDTelemetryCaptureCoordinator
    private let telemetryIngestClient: TelemetryIngestClient
    private let sessionProvider: () -> SessionContext
    private let appVersionProvider: () -> String
    private var cancellables: Set<AnyCancellable> = []

    init(
        bleClient: CoreBluetoothOBDClient,
        captureCoordinator: OBDTelemetryCaptureCoordinator,
        telemetryIngestClient: TelemetryIngestClient,
        sessionProvider: @escaping () -> SessionContext,
        appVersionProvider: @escaping () -> String
    ) {
        self.bleClient = bleClient
        self.captureCoordinator = captureCoordinator
        self.telemetryIngestClient = telemetryIngestClient
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
        if isCapturing {
            captureCoordinator.stopCapture()
        }
        bleClient.disconnect()
        uploadState = .idle
        statusMessage = "Adapter disconnected."
    }

    func toggleCapture() {
        if isCapturing {
            captureCoordinator.stopCapture()
            statusMessage = "Capture stopped with \(pendingRecordCount) pending records."
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

        uploadState = .uploading
        statusMessage = "Uploading telemetry batch..."

        Task {
            do {
                let response = try await telemetryIngestClient.upload(batch: batchBundle.request)
                captureCoordinator.clearPendingData(afterWindowEndedAt: batchBundle.windowEndedAt)

                let summary = "Accepted \(response.recordsAccepted)/\(response.recordsReceived) records."
                uploadState = .success(summary)
                if response.duplicate {
                    statusMessage = "\(summary) Server marked batch as duplicate."
                } else {
                    statusMessage = summary
                }
            } catch let backendError as BackendError {
                uploadState = .error(backendError.displayMessage)
                statusMessage = backendError.displayMessage
            } catch {
                uploadState = .error(error.localizedDescription)
                statusMessage = error.localizedDescription
            }
        }
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

                if !state.isConnected, isCapturing {
                    captureCoordinator.stopCapture()
                    statusMessage = "Capture stopped because adapter disconnected."
                }

                if case let .error(message) = state {
                    statusMessage = message
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
    }
}
