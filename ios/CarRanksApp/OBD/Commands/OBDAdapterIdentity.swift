import Foundation

/// Result of probing the adapter with `ATI` before running the full initialization handshake.
struct OBDAdapterIdentity: Equatable {
    let rawValue: String
    let normalizedValue: String
    let profile: OBDAdapterInitializationProfile

    static func fromATIResponse(_ rawResponse: String) -> OBDAdapterIdentity {
        let normalized = normalize(rawResponse)
        let uppercased = normalized.uppercased()

        let profile: OBDAdapterInitializationProfile
        if uppercased.contains("OBDLINK") || uppercased.contains("STN") {
            profile = .obdLink
        } else if uppercased.contains("ELM327") || uppercased.contains("ELM 327") {
            profile = .elm327
        } else {
            profile = .generic
        }

        return OBDAdapterIdentity(
            rawValue: rawResponse,
            normalizedValue: normalized,
            profile: profile
        )
    }

    private static func normalize(_ raw: String) -> String {
        raw
            .replacingOccurrences(of: ">", with: " ")
            .components(separatedBy: .newlines)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
            .joined(separator: " ")
    }
}
