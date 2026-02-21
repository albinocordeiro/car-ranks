import Foundation

/// Snapshot of active fault state captured from OBD readiness and stored DTC queries.
struct OBDDiagnosticSnapshot: Equatable {
    let observedAt: Date
    let milOn: Bool
    let dtcsActive: [String]

    init(
        observedAt: Date,
        milOn: Bool,
        dtcsActive: [String]
    ) {
        self.observedAt = observedAt
        self.milOn = milOn

        var normalized: [String] = []
        var seen: Set<String> = []
        for code in dtcsActive {
            let trimmed = code.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
            guard !trimmed.isEmpty else { continue }
            if seen.insert(trimmed).inserted {
                normalized.append(trimmed)
            }
        }
        self.dtcsActive = normalized
    }

    /// Used to suppress repeated diagnostics when the fault state has not changed.
    var stateSignature: String {
        "\(milOn ? 1 : 0)|\(dtcsActive.joined(separator: ","))"
    }
}
