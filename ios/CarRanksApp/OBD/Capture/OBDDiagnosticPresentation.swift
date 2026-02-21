import Foundation

/// View-facing diagnostic state summary derived from the latest polled OBD fault snapshot.
struct OBDDiagnosticPresentation: Equatable {
    let milIsOn: Bool?
    let milStatusText: String
    let activeDTCs: [String]
    let activeDTCsSummary: String
    let lastObservedText: String
    let changeMarkerText: String
    let isChangeMarkerHighlighted: Bool

    static func from(
        latestSnapshot: OBDDiagnosticSnapshot?,
        lastChangedAt: Date?
    ) -> OBDDiagnosticPresentation {
        guard let latestSnapshot else {
            return OBDDiagnosticPresentation(
                milIsOn: nil,
                milStatusText: "Unknown",
                activeDTCs: [],
                activeDTCsSummary: "None",
                lastObservedText: "Not observed yet",
                changeMarkerText: "No diagnostic state changes recorded yet.",
                isChangeMarkerHighlighted: false
            )
        }

        let activeDTCsSummary: String
        if latestSnapshot.dtcsActive.isEmpty {
            activeDTCsSummary = "None"
        } else {
            activeDTCsSummary = "\(latestSnapshot.dtcsActive.count) active"
        }

        let changeMarkerText: String
        let isChangeMarkerHighlighted: Bool
        if let lastChangedAt {
            changeMarkerText = "Diagnostic state changed at \(TelemetryTimestampFormatter.string(from: lastChangedAt))"
            isChangeMarkerHighlighted = true
        } else {
            changeMarkerText = "No diagnostic state changes detected during this capture."
            isChangeMarkerHighlighted = false
        }

        return OBDDiagnosticPresentation(
            milIsOn: latestSnapshot.milOn,
            milStatusText: latestSnapshot.milOn ? "On" : "Off",
            activeDTCs: latestSnapshot.dtcsActive,
            activeDTCsSummary: activeDTCsSummary,
            lastObservedText: TelemetryTimestampFormatter.string(from: latestSnapshot.observedAt),
            changeMarkerText: changeMarkerText,
            isChangeMarkerHighlighted: isChangeMarkerHighlighted
        )
    }
}
