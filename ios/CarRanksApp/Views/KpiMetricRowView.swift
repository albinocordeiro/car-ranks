import SwiftUI

/// Single KPI row, intentionally isolated so metric rendering can evolve independently.
struct KpiMetricRowView: View {
    let metric: KpiMetric

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(metric.kpiKey.replacingOccurrences(of: "_", with: " ").capitalized)
                .font(.headline)
            HStack {
                Text(metric.formattedValue)
                    .font(.title3.weight(.bold))
                Text(metric.unit)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
            HStack(spacing: 10) {
                Label(metric.direction.replacingOccurrences(of: "_", with: " "), systemImage: "arrow.up.and.down")
                Label("\(metric.sampleCount) samples", systemImage: "chart.bar")
                Label(metric.confidenceLevel.capitalized, systemImage: "checkmark.shield")
            }
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        .padding(.vertical, 6)
    }
}

private extension KpiMetric {
    var formattedValue: String {
        ValueFormatter.shared.string(from: NSNumber(value: value)) ?? String(format: "%.2f", value)
    }
}

private enum ValueFormatter {
    static let shared: NumberFormatter = {
        let formatter = NumberFormatter()
        formatter.maximumFractionDigits = 2
        formatter.minimumFractionDigits = 0
        formatter.numberStyle = .decimal
        return formatter
    }()
}
