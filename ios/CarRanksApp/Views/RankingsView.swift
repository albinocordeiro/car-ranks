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
                Text("Cohort Size: \(response.cohort.cohortSize)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Text("Sample Gate: \(response.cohort.sampleGatePassed ? "passed" : "not ready")")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                CohortKeyRowView(cohortKey: response.cohort.cohortKey)
                CohortFiltersSummaryView(filters: response.filters)
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
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("#\(row.rank)")
                    .font(.headline.weight(.bold))
                Spacer()
                Text("Score \(row.score.formattedScore)")
                    .font(.subheadline.weight(.semibold))
                    .monospacedDigit()
            }

            VStack(alignment: .leading, spacing: 2) {
                Text("Vehicle UID")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
                Text(row.vehicleUID.uuidString.lowercased())
                    .font(.system(.footnote, design: .monospaced))
                    .textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
            }

            Text("Confidence: \(row.confidenceLevel.readableIdentifier)")
                .font(.caption)
                .foregroundStyle(.secondary)

            if row.kpis.isEmpty {
                Text("No KPI breakdown")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 6) {
                    Text("KPI Breakdown")
                        .font(.caption.weight(.semibold))
                    ForEach(row.sortedKpis, id: \.key) { item in
                        HStack(alignment: .firstTextBaseline, spacing: 8) {
                            Text(item.key.readableIdentifier)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Spacer()
                            Text(item.value.formattedScore)
                                .font(.caption.weight(.semibold))
                                .foregroundStyle(.secondary)
                                .monospacedDigit()
                        }
                    }
                }
            }
        }
        .padding(.vertical, 6)
    }
}

private struct CohortKeyRowView: View {
    let cohortKey: String

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Cohort Key")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
            Text(cohortKey)
                .font(.system(.footnote, design: .monospaced))
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(.vertical, 2)
    }
}

private struct CohortFiltersSummaryView: View {
    let filters: RankingsResponse.Filters

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Cohort Filters")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)

            ForEach(Array(filterRows.enumerated()), id: \.offset) { _, row in
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    Text(row.label)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Spacer()
                    Text(row.value)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.trailing)
                }
            }
        }
        .padding(.vertical, 2)
    }

    private var filterRows: [(label: String, value: String)] {
        [
            ("Powertrain", normalizedFilterValue(filters.powertrainClass)?.uppercased() ?? "Any"),
            ("Make", normalizedFilterValue(filters.make) ?? "Any"),
            ("Model", normalizedFilterValue(filters.model) ?? "Any"),
            ("Trim", normalizedFilterValue(filters.trim) ?? "Any"),
            ("Year Band", normalizedFilterValue(filters.yearBand) ?? "Any"),
            ("Region", normalizedFilterValue(filters.region) ?? "Any"),
        ]
    }

    private func normalizedFilterValue(_ value: String?) -> String? {
        guard let value else { return nil }
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, trimmed.lowercased() != "unknown" else {
            return nil
        }
        return trimmed
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
