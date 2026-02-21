import Foundation

enum DataSourceMode: String, CaseIterable, Codable, Identifiable {
    case mock
    case live

    var id: String { rawValue }

    var displayName: String {
        switch self {
        case .mock: return "Mock"
        case .live: return "Live"
        }
    }
}
