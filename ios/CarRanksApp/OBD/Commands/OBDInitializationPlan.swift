import Foundation

/// One setup command in the adapter handshake plan.
struct OBDInitializationCommandStep: Equatable {
    let command: String
    let isRequired: Bool
    let purpose: String
}

/// Central place for handshake command plans so they are easy to audit and tweak per adapter family.
enum OBDInitializationPlan {
    static func steps(for profile: OBDAdapterInitializationProfile) -> [OBDInitializationCommandStep] {
        switch profile {
        case .obdLink:
            return [
                .init(command: "ATWS", isRequired: true, purpose: "warm start for STN/OBDLink firmware"),
                .init(command: "ATE0", isRequired: true, purpose: "disable echo"),
                .init(command: "ATL0", isRequired: true, purpose: "disable line feeds"),
                .init(command: "ATS0", isRequired: true, purpose: "disable spaces"),
                .init(command: "ATH0", isRequired: true, purpose: "disable headers"),
                .init(command: "ATSP0", isRequired: true, purpose: "enable auto protocol selection"),
                .init(command: "ATAT1", isRequired: false, purpose: "enable adaptive timing"),
                .init(command: "ATAL", isRequired: false, purpose: "allow long frames when supported"),
            ]
        case .elm327:
            return [
                .init(command: "ATZ", isRequired: true, purpose: "full reset for ELM-compatible adapters"),
                .init(command: "ATE0", isRequired: true, purpose: "disable echo"),
                .init(command: "ATL0", isRequired: true, purpose: "disable line feeds"),
                .init(command: "ATS0", isRequired: true, purpose: "disable spaces"),
                .init(command: "ATH0", isRequired: true, purpose: "disable headers"),
                .init(command: "ATSP0", isRequired: true, purpose: "enable auto protocol selection"),
                .init(command: "ATAT1", isRequired: false, purpose: "enable adaptive timing"),
                .init(command: "ATAL", isRequired: false, purpose: "allow long frames when supported"),
            ]
        case .generic:
            return [
                .init(command: "ATZ", isRequired: true, purpose: "adapter reset"),
                .init(command: "ATE0", isRequired: true, purpose: "disable echo"),
                .init(command: "ATL0", isRequired: true, purpose: "disable line feeds"),
                .init(command: "ATS0", isRequired: true, purpose: "disable spaces"),
                .init(command: "ATH0", isRequired: true, purpose: "disable headers"),
                .init(command: "ATSP0", isRequired: true, purpose: "enable auto protocol selection"),
                .init(command: "ATAT1", isRequired: false, purpose: "enable adaptive timing when available"),
            ]
        }
    }
}
