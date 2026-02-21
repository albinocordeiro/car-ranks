import SwiftUI

struct KpiReadinessScreen: View {
    @ObservedObject private var environment: AppEnvironment
    @StateObject private var viewModel: KpiReadinessViewModel

    init(environment: AppEnvironment) {
        self.environment = environment
        _viewModel = StateObject(
            wrappedValue: KpiReadinessViewModel(
                captureScenario: environment.activeCaptureScenario,
                sessionProvider: { environment.sessionContext },
                backendClient: environment.makeBackendClient()
            )
        )
    }

    var body: some View {
        KpiReadinessView(viewModel: viewModel)
            .navigationTitle("KPI Readiness")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Refresh") {
                        viewModel.refresh()
                    }
                    .accessibilityIdentifier("kpi-readiness-refresh-button")
                }
            }
            .onAppear {
                viewModel.loadIfNeeded()
            }
            .onChange(of: environment.sessionContext) { _, _ in
                viewModel.refresh()
            }
            .onChange(of: environment.dataSourceMode) { _, _ in
                viewModel.refresh()
            }
    }
}

struct KpiReadinessView: View {
    @ObservedObject var viewModel: KpiReadinessViewModel

    var body: some View {
        Group {
            switch viewModel.state {
            case .idle, .loading:
                LoadingKpiReadinessView()
                    .accessibilityIdentifier("kpi-readiness-loading-state")
            case let .success(response):
                SuccessKpiReadinessView(response: response)
                    .accessibilityIdentifier("kpi-readiness-success-state")
            case .empty:
                EmptyKpiReadinessView()
                    .accessibilityIdentifier("kpi-readiness-empty-state")
            case let .error(message):
                ErrorKpiReadinessView(message: message) {
                    viewModel.refresh()
                }
                .accessibilityIdentifier("kpi-readiness-error-state")
            }
        }
    }
}

private struct LoadingKpiReadinessView: View {
    var body: some View {
        VStack(spacing: 14) {
            ProgressView()
                .controlSize(.large)
            Text("Loading readiness status...")
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

private struct SuccessKpiReadinessView: View {
    let response: ReadinessResponse

    var body: some View {
        List {
            Section("Readiness Snapshot") {
                Text("Vehicle: \(response.vehicleUID.uuidString.lowercased())")
                    .font(.system(.footnote, design: .monospaced))
                    .textSelection(.enabled)
                Text("Generated: \(response.generatedAt)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Text("Timeframe: \(response.timeframe)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }

            Section("Readiness Families") {
                ForEach(Array(response.families.enumerated()), id: \.offset) { _, family in
                    ReadinessFamilyRowView(family: family)
                }
            }
        }
        .listStyle(.insetGrouped)
    }
}

private struct ReadinessFamilyRowView: View {
    let family: ReadinessFamily

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(family.rankingType.readableIdentifier)
                .font(.headline)
            HStack(spacing: 10) {
                Label(family.status.readableIdentifier, systemImage: "checkmark.circle")
                Label("\(family.sampleCount) samples", systemImage: "chart.bar")
                Label(family.confidenceLevel.capitalized, systemImage: "checkmark.shield")
            }
            .font(.caption)
            .foregroundStyle(.secondary)

            if family.missingRequirements.isEmpty {
                Text("No missing requirements")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Missing requirements")
                        .font(.caption.weight(.semibold))
                    ForEach(family.missingRequirements, id: \.self) { requirement in
                        Text("• \(requirement)")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
            }
        }
        .padding(.vertical, 6)
    }
}

private struct EmptyKpiReadinessView: View {
    var body: some View {
        ContentUnavailableView(
            "No Readiness Data",
            systemImage: "tray",
            description: Text("No readiness families were returned for this vehicle and timeframe.")
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

private struct ErrorKpiReadinessView: View {
    let message: String
    let onRetry: () -> Void

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "exclamationmark.triangle")
                .imageScale(.large)
            Text("Failed to load readiness status")
                .font(.headline)
            Text(message)
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)
            Button("Retry", action: onRetry)
                .buttonStyle(.borderedProminent)
                .accessibilityIdentifier("kpi-readiness-retry-button")
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

private extension String {
    var readableIdentifier: String {
        replacingOccurrences(of: "_", with: " ").capitalized
    }
}
