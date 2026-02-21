import SwiftUI

struct KpiTemperatureImpactScreen: View {
    @ObservedObject private var environment: AppEnvironment
    @StateObject private var viewModel: KpiTemperatureImpactViewModel

    init(environment: AppEnvironment) {
        self.environment = environment
        _viewModel = StateObject(
            wrappedValue: KpiTemperatureImpactViewModel(
                captureScenario: environment.activeCaptureScenario,
                sessionProvider: { environment.sessionContext },
                backendClient: environment.makeBackendClient()
            )
        )
    }

    var body: some View {
        KpiTemperatureImpactView(viewModel: viewModel)
            .navigationTitle("KPI Temperature")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Refresh") {
                        viewModel.refresh()
                    }
                    .accessibilityIdentifier("kpi-temperature-impact-refresh-button")
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

struct KpiTemperatureImpactView: View {
    @ObservedObject var viewModel: KpiTemperatureImpactViewModel

    var body: some View {
        Group {
            switch viewModel.state {
            case .idle, .loading:
                LoadingKpiTemperatureImpactView()
                    .accessibilityIdentifier("kpi-temperature-impact-loading-state")
            case let .success(response):
                SuccessKpiTemperatureImpactView(response: response)
                    .accessibilityIdentifier("kpi-temperature-impact-success-state")
            case .empty:
                EmptyKpiTemperatureImpactView()
                    .accessibilityIdentifier("kpi-temperature-impact-empty-state")
            case let .error(message):
                ErrorKpiTemperatureImpactView(message: message) {
                    viewModel.refresh()
                }
                .accessibilityIdentifier("kpi-temperature-impact-error-state")
            }
        }
    }
}

private struct LoadingKpiTemperatureImpactView: View {
    var body: some View {
        VStack(spacing: 14) {
            ProgressView()
                .controlSize(.large)
            Text("Loading temperature impact...")
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

private struct SuccessKpiTemperatureImpactView: View {
    let response: TemperatureImpactResponse

    var body: some View {
        List {
            Section("Temperature Impact Snapshot") {
                Text("Vehicle: \(response.vehicleUID.uuidString.lowercased())")
                    .font(.system(.footnote, design: .monospaced))
                    .textSelection(.enabled)
                Text("Generated: \(response.generatedAt)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Text("Baseline: \(response.baselineTemperatureBin)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Text("Compare: \(response.compareTemperatureBin)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Text("Cohort Size: \(response.cohortBenchmark.cohortSize)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }

            Section("Metrics") {
                ForEach(Array(response.metrics.enumerated()), id: \.offset) { _, metric in
                    KpiMetricRowView(metric: metric)
                }
            }

            Section("Cohort Benchmark Percentiles") {
                PercentileRowView(
                    title: "Cold Weather Range Retention",
                    value: response.cohortBenchmark.percentiles.coldWeatherRangeRetention
                )
                PercentileRowView(
                    title: "Range Temperature Sensitivity Index",
                    value: response.cohortBenchmark.percentiles.rangeTemperatureSensitivityIndex
                )
                PercentileRowView(
                    title: "Cold Weather Charge Speed Retention",
                    value: response.cohortBenchmark.percentiles.coldWeatherChargeSpeedRetention
                )
            }
        }
        .listStyle(.insetGrouped)
    }
}

private struct PercentileRowView: View {
    let title: String
    let value: Double?

    var body: some View {
        HStack {
            Text(title)
            Spacer()
            Text(value.formattedPercentile)
                .font(.subheadline.weight(.semibold))
        }
    }
}

private struct EmptyKpiTemperatureImpactView: View {
    var body: some View {
        ContentUnavailableView(
            "No Temperature Impact Data",
            systemImage: "tray",
            description: Text("No temperature impact metrics were returned for this vehicle and timeframe.")
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

private struct ErrorKpiTemperatureImpactView: View {
    let message: String
    let onRetry: () -> Void

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "exclamationmark.triangle")
                .imageScale(.large)
            Text("Failed to load temperature impact")
                .font(.headline)
            Text(message)
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)
            Button("Retry", action: onRetry)
                .buttonStyle(.borderedProminent)
                .accessibilityIdentifier("kpi-temperature-impact-retry-button")
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

private extension Optional where Wrapped == Double {
    var formattedPercentile: String {
        guard let value = self else { return "Unavailable" }
        return String(format: "%.0fth", value)
    }
}
