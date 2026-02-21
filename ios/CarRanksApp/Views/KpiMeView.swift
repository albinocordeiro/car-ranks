import SwiftUI

struct KpiMeScreen: View {
    @ObservedObject private var environment: AppEnvironment
    @StateObject private var viewModel: KpiMeViewModel

    init(environment: AppEnvironment) {
        self.environment = environment
        _viewModel = StateObject(
            wrappedValue: KpiMeViewModel(
                captureScenario: environment.activeCaptureScenario,
                sessionProvider: { environment.sessionContext },
                backendClient: environment.makeBackendClient()
            )
        )
    }

    var body: some View {
        KpiMeView(viewModel: viewModel)
            .navigationTitle("KPI Me")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Refresh") {
                        viewModel.refresh()
                    }
                    .accessibilityIdentifier("kpi-me-refresh-button")
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

struct KpiMeView: View {
    @ObservedObject var viewModel: KpiMeViewModel

    var body: some View {
        Group {
            switch viewModel.state {
            case .idle, .loading:
                LoadingKpiMeView()
                    .accessibilityIdentifier("kpi-me-loading-state")
            case let .success(response):
                SuccessKpiMeView(response: response)
                    .accessibilityIdentifier("kpi-me-success-state")
            case .empty:
                EmptyKpiMeView()
                    .accessibilityIdentifier("kpi-me-empty-state")
            case let .error(message):
                ErrorKpiMeView(message: message) {
                    viewModel.refresh()
                }
                .accessibilityIdentifier("kpi-me-error-state")
            }
        }
    }
}

private struct LoadingKpiMeView: View {
    var body: some View {
        VStack(spacing: 14) {
            ProgressView()
                .controlSize(.large)
            Text("Loading KPI snapshot...")
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

private struct SuccessKpiMeView: View {
    let response: GenericKpiResponse

    var body: some View {
        List {
            Section("Snapshot") {
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

private struct EmptyKpiMeView: View {
    var body: some View {
        ContentUnavailableView(
            "No KPI Data",
            systemImage: "tray",
            description: Text("No KPI snapshot was returned for this vehicle and timeframe.")
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

private struct ErrorKpiMeView: View {
    let message: String
    let onRetry: () -> Void

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "exclamationmark.triangle")
                .imageScale(.large)
            Text("Failed to load KPI snapshot")
                .font(.headline)
            Text(message)
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)
            Button("Retry", action: onRetry)
                .buttonStyle(.borderedProminent)
                .accessibilityIdentifier("kpi-me-retry-button")
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}
