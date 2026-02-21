import Foundation

enum OBDSignalRecordStatus: String, Codable {
    case ok
    case stale
    case unavailable
    case notSupported = "not_supported"
    case permissionDenied = "permission_denied"
    case error
}

/// Internal signal observation model kept separate from API payload for clarity and testing.
struct OBDSignalRecord: Identifiable, Equatable {
    let id: UUID
    let observedAt: Date
    let signalKey: String
    let valueNumber: Double?
    let unit: String?
    let status: OBDSignalRecordStatus
    let confidence: Double?
    let sourceSignal: String?

    init(
        id: UUID = UUID(),
        observedAt: Date,
        signalKey: String,
        valueNumber: Double?,
        unit: String?,
        status: OBDSignalRecordStatus,
        confidence: Double?,
        sourceSignal: String?
    ) {
        self.id = id
        self.observedAt = observedAt
        self.signalKey = signalKey
        self.valueNumber = valueNumber
        self.unit = unit
        self.status = status
        self.confidence = confidence
        self.sourceSignal = sourceSignal
    }
}
