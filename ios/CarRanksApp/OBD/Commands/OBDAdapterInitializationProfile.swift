import Foundation

/// High-level adapter families we can target with slightly different init strategies.
enum OBDAdapterInitializationProfile: String, Equatable {
    case obdLink
    case elm327
    case generic

    static func fallbackOrder(preferred: OBDAdapterInitializationProfile?) -> [OBDAdapterInitializationProfile] {
        switch preferred {
        case .obdLink:
            return [.obdLink, .elm327, .generic]
        case .elm327:
            return [.elm327, .generic]
        case .generic, .none:
            return [.generic, .elm327]
        }
    }
}
