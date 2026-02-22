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

    static func isLikelyOBDAdapter(name: String, advertisedServiceUUIDs: [String]) -> Bool {
        let uppercaseName = name.uppercased()
        let candidateServices = Set(OBDBLEConstants.candidateServiceUUIDs.map { $0.uuidString.uppercased() })
        let advertisedServices = Set(advertisedServiceUUIDs.map { $0.uppercased() })

        if !advertisedServices.intersection(candidateServices).isEmpty {
            return true
        }

        if excludedNameTokens.contains(where: { uppercaseName.contains($0) }) {
            return false
        }

        if uppercaseName == "UNNAMED OBD ADAPTER" {
            return true
        }

        return likelyNameTokens.contains(where: { uppercaseName.contains($0) })
    }
}
