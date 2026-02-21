import SwiftUI

struct OBDCaptureScreen: View {
    @ObservedObject private var environment: AppEnvironment
    @StateObject private var viewModel: OBDCaptureViewModel

    init(environment: AppEnvironment) {
        self.environment = environment
        let bleClient = environment.obdBLEClient
        let commandExecutor = OBDCommandExecutor(transport: bleClient)
        let captureCoordinator = OBDTelemetryCaptureCoordinator(commandExecutor: commandExecutor)
        _viewModel = StateObject(
            wrappedValue: OBDCaptureViewModel(
                bleClient: bleClient,
                captureCoordinator: captureCoordinator,
                uploadQueueCoordinator: environment.telemetryUploadQueue,
                sessionProvider: { environment.sessionContext },
                appVersionProvider: {
                    Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "0.0.0"
                }
            )
        )
    }

    var body: some View {
        OBDCaptureView(viewModel: viewModel)
            .navigationTitle("OBD Capture")
            .onChange(of: environment.sessionContext) { _, _ in
                // Session changes affect upload headers and vehicle mapping only.
            }
    }
}

struct OBDCaptureView: View {
    @ObservedObject var viewModel: OBDCaptureViewModel

    var body: some View {
        List {
            connectionSection
            adaptersSection
            captureSection
            diagnosticsSection
            uploadSection
            recentRecordsSection
        }
        .listStyle(.insetGrouped)
        .accessibilityIdentifier("obd-capture-screen")
    }

    private var connectionSection: some View {
        Section("Connection") {
            Text(viewModel.connectionState.statusText)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(connectionStatusColor)
                .accessibilityIdentifier("obd-connection-status")

            Text(viewModel.statusMessage)
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("obd-status-message")

            LabeledContent("Adapter Identity", value: viewModel.adapterIdentitySummary)
                .font(.caption)
                .accessibilityIdentifier("obd-adapter-identity")

            LabeledContent("Init Profile", value: viewModel.initializationProfileSummary)
                .font(.caption)
                .accessibilityIdentifier("obd-init-profile")

            HStack {
                Button("Scan") {
                    viewModel.startScanning()
                }
                .buttonStyle(.bordered)
                .accessibilityIdentifier("obd-scan-button")

                Button("Stop Scan") {
                    viewModel.stopScanning()
                }
                .buttonStyle(.bordered)
                .accessibilityIdentifier("obd-stop-scan-button")

                Button("Disconnect") {
                    viewModel.disconnect()
                }
                .buttonStyle(.bordered)
                .accessibilityIdentifier("obd-disconnect-button")
            }
        }
    }

    private var adaptersSection: some View {
        Section("Discovered Adapters") {
            if viewModel.discoveredDevices.isEmpty {
                Text("No adapters discovered yet.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(viewModel.discoveredDevices) { adapter in
                    VStack(alignment: .leading, spacing: 6) {
                        Text(adapter.name)
                            .font(.subheadline.weight(.semibold))
                        Text("RSSI: \(adapter.rssi)dBm")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        if !adapter.advertisedServices.isEmpty {
                            Text("Services: \(adapter.advertisedServices.joined(separator: ", "))")
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                                .textSelection(.enabled)
                        }
                        Button("Connect") {
                            viewModel.connect(to: adapter.id)
                        }
                        .buttonStyle(.borderedProminent)
                        .accessibilityIdentifier("obd-connect-\(adapter.id.uuidString.lowercased())")
                    }
                    .padding(.vertical, 4)
                }
            }
        }
    }

    private var captureSection: some View {
        Section("Capture") {
            HStack {
                Text("Sample Interval (s)")
                Spacer()
                TextField("5", text: $viewModel.sampleIntervalSecondsText)
                    .keyboardType(.numberPad)
                    .multilineTextAlignment(.trailing)
                    .frame(width: 90)
                    .textFieldStyle(.roundedBorder)
                    .accessibilityIdentifier("obd-sample-interval-field")
            }

            HStack {
                Button(viewModel.isCapturing ? "Stop Capture" : "Start Capture") {
                    viewModel.toggleCapture()
                }
                .buttonStyle(.borderedProminent)
                .accessibilityIdentifier("obd-toggle-capture-button")

                Spacer()
                VStack(alignment: .trailing, spacing: 2) {
                    Text("Signals: \(viewModel.pendingRecordCount)")
                        .accessibilityIdentifier("obd-pending-records")
                    Text("Diagnostics: \(viewModel.pendingDiagnosticCount)")
                        .accessibilityIdentifier("obd-pending-diagnostics")
                    Text("Session events: \(viewModel.pendingSessionEventCount)")
                        .accessibilityIdentifier("obd-pending-session-events")
                }
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
                .monospacedDigit()
            }
        }
    }

    private var uploadSection: some View {
        Section("Upload") {
            Button("Upload Pending Batch") {
                viewModel.uploadPendingBatch()
            }
            .buttonStyle(.borderedProminent)
            .accessibilityIdentifier("obd-upload-button")

            Button("Retry Queued Uploads") {
                viewModel.retryQueuedUploads()
            }
            .buttonStyle(.bordered)
            .accessibilityIdentifier("obd-retry-queued-button")

            Text("Queued batches: \(viewModel.queuedBatchCount)")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
                .accessibilityIdentifier("obd-queued-batch-count")

            Text("Pending payload: \(viewModel.pendingUploadSummary.inlineDescription)")
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("obd-pending-upload-summary")

            Text("Last queued: \(viewModel.lastQueuedBatchSummary)")
                .font(.caption2)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("obd-last-queued-summary")

            Text("Last uploaded: \(viewModel.lastSuccessfulUploadSummary)")
                .font(.caption2)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("obd-last-uploaded-summary")

            switch viewModel.uploadState {
            case .idle:
                EmptyView()
            case .uploading:
                Label("Uploading...", systemImage: "arrow.up.circle")
                    .font(.caption)
            case let .success(message):
                Label(message, systemImage: "checkmark.circle")
                    .font(.caption)
                    .foregroundStyle(.green)
                    .fixedSize(horizontal: false, vertical: true)
            case let .error(message):
                Label(message, systemImage: "xmark.octagon")
                    .font(.caption)
                    .foregroundStyle(.red)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    private var diagnosticsSection: some View {
        Section("Diagnostics") {
            LabeledContent("MIL") {
                Text(viewModel.diagnosticPresentation.milStatusText)
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(milStatusColor)
            }
            .accessibilityIdentifier("obd-diagnostics-mil")

            LabeledContent("Active DTCs", value: viewModel.diagnosticPresentation.activeDTCsSummary)
                .font(.caption)
                .accessibilityIdentifier("obd-diagnostics-dtc-summary")

            LabeledContent("Last Observed", value: viewModel.diagnosticPresentation.lastObservedText)
                .font(.caption)
                .accessibilityIdentifier("obd-diagnostics-last-observed")

            Label(
                viewModel.diagnosticPresentation.changeMarkerText,
                systemImage: viewModel.diagnosticPresentation.isChangeMarkerHighlighted
                    ? "dot.radiowaves.left.and.right"
                    : "dot.radiowaves.left.and.right.slash"
            )
            .font(.caption)
            .foregroundStyle(viewModel.diagnosticPresentation.isChangeMarkerHighlighted ? .orange : .secondary)
            .fixedSize(horizontal: false, vertical: true)
            .accessibilityIdentifier("obd-diagnostics-change-marker")

            if !viewModel.diagnosticPresentation.activeDTCs.isEmpty {
                ForEach(viewModel.diagnosticPresentation.activeDTCs, id: \.self) { code in
                    Text(code)
                        .font(.caption.monospaced())
                        .accessibilityIdentifier("obd-diagnostics-dtc-\(code)")
                }
            }
        }
    }

    private var recentRecordsSection: some View {
        Section("Recent Records") {
            if viewModel.recentRecords.isEmpty {
                Text("No records captured yet.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(viewModel.recentRecords) { record in
                    VStack(alignment: .leading, spacing: 4) {
                        Text(record.signalKey)
                            .font(.caption.weight(.semibold))
                        HStack {
                            Text(record.status.rawValue)
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                            Spacer()
                            Text(record.valueSummary)
                                .font(.caption.weight(.semibold))
                                .monospacedDigit()
                        }
                        Text(TelemetryTimestampFormatter.string(from: record.observedAt))
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                    .padding(.vertical, 2)
                }
            }
        }
    }

    private var connectionStatusColor: Color {
        switch viewModel.connectionState {
        case .connected:
            return .green
        case .reconnecting:
            return .orange
        case .error:
            return .red
        default:
            return .primary
        }
    }

    private var milStatusColor: Color {
        switch viewModel.diagnosticPresentation.milIsOn {
        case true:
            return .red
        case false:
            return .green
        case nil:
            return .secondary
        }
    }
}

private extension OBDSignalRecord {
    var valueSummary: String {
        guard let valueNumber, let unit else {
            return "n/a"
        }

        if abs(valueNumber) >= 100 {
            return String(format: "%.0f %@", valueNumber, unit)
        }
        return String(format: "%.2f %@", valueNumber, unit)
    }
}
