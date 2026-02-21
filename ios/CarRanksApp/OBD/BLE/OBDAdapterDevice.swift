import Foundation

/// Lightweight projection used by the UI so CoreBluetooth internals stay encapsulated.
struct OBDAdapterDevice: Identifiable, Equatable {
    let id: UUID
    let name: String
    let rssi: Int
    let advertisedServices: [String]
    let lastSeenAt: Date
}
