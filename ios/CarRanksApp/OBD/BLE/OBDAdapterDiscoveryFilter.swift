import Foundation

/// Heuristic filter that keeps discovery focused on likely OBD adapters for MVP UX.
enum OBDAdapterDiscoveryFilter {
    private static let likelyNameTokens: [String] = [
        "OBD",
        "ELM",
        "VEEPEAK",
        "OBDLINK",
        "VLINKER",
        "V-LINK",
        "VGATE",
        "BAFX",
        "CARISTA",
        "KONNWEI",
        "NEXAS",
        "AUTOPHIX",
        "FIXD",
    ]

    private static let excludedNameTokens: [String] = [
        "MACBOOK",
        "IPHONE",
        "IPAD",
        "AIRPODS",
        "APPLE WATCH",
        "HOMEPOD",
        "BEATS",
    ]

    static func isLikelyOBDAdapter(
        name: String,
        advertisedServiceUUIDs: [String],
        isConnectable: Bool = true
    ) -> Bool {
        // Ignore broadcast-only peripherals because we cannot connect to them anyway.
        guard isConnectable else {
            return false
        }

        let normalizedName = name.trimmingCharacters(in: .whitespacesAndNewlines)
        let uppercaseName = normalizedName.uppercased()
        let candidateServices = Set(OBDBLEConstants.candidateServiceUUIDs.map { $0.uuidString.uppercased() })
        let advertisedServices = Set(advertisedServiceUUIDs.map { $0.uppercased() })
        let hasCandidateService = !advertisedServices.intersection(candidateServices).isEmpty
        let isPlaceholderName = uppercaseName.isEmpty || uppercaseName == "UNNAMED OBD ADAPTER"
        let isExcludedName = excludedNameTokens.contains(where: { uppercaseName.contains($0) })
        let hasLikelyNameToken = !isPlaceholderName && likelyNameTokens.contains(where: { uppercaseName.contains($0) })

        // Avoid showing obvious personal devices even if they advertise common UART services.
        if isExcludedName && !hasLikelyNameToken {
            return false
        }

        if hasLikelyNameToken {
            return true
        }

        // Unknown names must advertise a known UART service to be shown.
        if isPlaceholderName {
            return hasCandidateService
        }

        return hasCandidateService
    }
}
