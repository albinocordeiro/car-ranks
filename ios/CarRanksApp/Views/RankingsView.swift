import SwiftUI

struct RankingsScreen: View {
    @ObservedObject private var environment: AppEnvironment
    @StateObject private var viewModel: RankingsViewModel

    init(environment: AppEnvironment) {
        self.environment = environment
        _viewModel = StateObject(
            wrappedValue: RankingsViewModel(
                captureScenario: environment.activeCaptureScenario,
                backendClient: environment.makeBackendClient()
            )
        )
    }

    var body: some View {
        RankingsView(viewModel: viewModel)
            .navigationTitle("Rankings")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Refresh") {
                        viewModel.refresh()
                    }
                    .accessibilityIdentifier("rankings-refresh-button")
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

struct RankingsView: View {
    @ObservedObject var viewModel: RankingsViewModel

    var body: some View {
        Group {
            switch viewModel.state {
            case .idle, .loading:
                LoadingRankingsView()
                    .accessibilityIdentifier("rankings-loading-state")
            case let .success(response):
                SuccessRankingsView(response: response)
                    .accessibilityIdentifier("rankings-success-state")
            case .empty:
                EmptyRankingsView()
                    .accessibilityIdentifier("rankings-empty-state")
            case let .error(message):
                ErrorRankingsView(message: message) {
                    viewModel.refresh()
                }
                .accessibilityIdentifier("rankings-error-state")
            }
        }
    }
}

private struct LoadingRankingsView: View {
    var body: some View {
        VStack(spacing: 14) {
            ProgressView()
                .controlSize(.large)
            Text("Loading rankings...")
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

private struct SuccessRankingsView: View {
    let response: RankingsResponse

    var body: some View {
        List {
            Section("Ranking Snapshot") {
                Text("Generated: \(response.generatedAt)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Text("Type: \(response.rankingType)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Text("Timeframe: \(response.timeframe)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Text("Temperature: \(response.temperatureBin)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Text("Cohort: \(response.cohort.cohortKey)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Text("Cohort Size: \(response.cohort.cohortSize)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }

            Section("Ranked Vehicles") {
                ForEach(Array(response.rows.enumerated()), id: \.offset) { _, row in
                    RankingRowView(row: row)
                }
            }
        }
        .listStyle(.insetGrouped)
    }
}

private struct RankingRowView: View {
    let row: RankingsResponse.Row

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("#\(row.rank)")
                    .font(.headline.weight(.bold))
                Spacer()
                Text("Score \(row.score.formattedScore)")
                    .font(.subheadline.weight(.semibold))
            }

            Text("Vehicle: \(row.vehicleUID.uuidString.lowercased())")
                .font(.system(.footnote, design: .monospaced))
                .textSelection(.enabled)

            Text("Confidence: \(row.confidenceLevel.readableIdentifier)")
                .font(.caption)
                .foregroundStyle(.secondary)

            if row.kpis.isEmpty {
                Text("No KPI breakdown")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 2) {
                    Text("KPI Breakdown")
                        .font(.caption.weight(.semibold))
                    ForEach(row.sortedKpis, id: \.key) { item in
                        Text("• \(item.key.readableIdentifier): \(item.value.formattedScore)")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
            }
        }
        .padding(.vertical, 6)
    }
}

private struct EmptyRankingsView: View {
    var body: some View {
        ContentUnavailableView(
            "No Rankings Data",
            systemImage: "tray",
            description: Text("No ranked vehicles were returned for this query.")
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

private struct ErrorRankingsView: View {
    let message: String
    let onRetry: () -> Void

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "exclamationmark.triangle")
                .imageScale(.large)
            Text("Failed to load rankings")
                .font(.headline)
            Text(message)
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)
            Button("Retry", action: onRetry)
                .buttonStyle(.borderedProminent)
                .accessibilityIdentifier("rankings-retry-button")
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

private extension RankingsResponse.Row {
    var sortedKpis: [(key: String, value: Double)] {
        kpis.sorted { lhs, rhs in
            lhs.key < rhs.key
        }
    }
}

private extension String {
    var readableIdentifier: String {
        replacingOccurrences(of: "_", with: " ").capitalized
    }
}

private extension Double {
    var formattedScore: String {
        String(format: "%.2f", self)
    }
}
