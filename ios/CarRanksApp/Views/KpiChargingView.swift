import SwiftUI

struct KpiChargingScreen: View {
    @ObservedObject private var environment: AppEnvironment
    @StateObject private var viewModel: KpiChargingViewModel

    init(environment: AppEnvironment) {
        self.environment = environment
        _viewModel = StateObject(
            wrappedValue: KpiChargingViewModel(
                captureScenario: environment.activeCaptureScenario,
                sessionProvider: { environment.sessionContext },
                backendClient: environment.makeBackendClient()
            )
        )
    }

    var body: some View {
        KpiChargingView(viewModel: viewModel)
            .navigationTitle("KPI Charging")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Refresh") {
                        viewModel.refresh()
                    }
                    .accessibilityIdentifier("kpi-charging-refresh-button")
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

struct KpiChargingView: View {
    @ObservedObject var viewModel: KpiChargingViewModel

    var body: some View {
        Group {
            switch viewModel.state {
            case .idle, .loading:
                LoadingKpiChargingView()
                    .accessibilityIdentifier("kpi-charging-loading-state")
            case let .success(response):
                SuccessKpiChargingView(response: response)
                    .accessibilityIdentifier("kpi-charging-success-state")
            case .empty:
                EmptyKpiChargingView()
                    .accessibilityIdentifier("kpi-charging-empty-state")
            case let .error(message):
                ErrorKpiChargingView(message: message) {
                    viewModel.refresh()
                }
                .accessibilityIdentifier("kpi-charging-error-state")
            }
        }
    }
}

private struct LoadingKpiChargingView: View {
    var body: some View {
        VStack(spacing: 14) {
            ProgressView()
                .controlSize(.large)
            Text("Loading charging KPI snapshot...")
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

private struct SuccessKpiChargingView: View {
    let response: GenericKpiResponse

    var body: some View {
        List {
            Section("Charging Snapshot") {
                Text("Vehicle: \(response.vehicleUID.uuidString.lowercased())")
                    .font(.system(.footnote, design: .monospaced))
                    .textSelection(.enabled)
                Text("Generated: \(response.generatedAt)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Text("Ranking: \(response.rankingType)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }

            Section("Metrics") {
                ForEach(Array(response.kpis.enumerated()), id: \.offset) { _, metric in
                    KpiMetricRowView(metric: metric)
                }
            }
        }
        .listStyle(.insetGrouped)
    }
}

private struct EmptyKpiChargingView: View {
    var body: some View {
        ContentUnavailableView(
            "No Charging KPI Data",
            systemImage: "tray",
            description: Text("No charging KPI snapshot was returned for this vehicle and timeframe.")
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

private struct ErrorKpiChargingView: View {
    let message: String
    let onRetry: () -> Void

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "exclamationmark.triangle")
                .imageScale(.large)
            Text("Failed to load charging KPI snapshot")
                .font(.headline)
            Text(message)
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)
            Button("Retry", action: onRetry)
                .buttonStyle(.borderedProminent)
                .accessibilityIdentifier("kpi-charging-retry-button")
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}
