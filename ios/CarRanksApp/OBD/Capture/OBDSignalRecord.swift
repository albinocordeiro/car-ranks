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
    let sessionID: UUID?
    let signalKey: String
    let valueNumber: Double?
    let unit: String?
    let status: OBDSignalRecordStatus
    let confidence: Double?
    let sourceSignal: String?
    let rawPayloadRef: String?

    init(
        id: UUID = UUID(),
        observedAt: Date,
        sessionID: UUID? = nil,
        signalKey: String,
        valueNumber: Double?,
        unit: String?,
        status: OBDSignalRecordStatus,
        confidence: Double?,
        sourceSignal: String?,
        rawPayloadRef: String? = nil
    ) {
        self.id = id
        self.observedAt = observedAt
        self.sessionID = sessionID
        self.signalKey = signalKey
        self.valueNumber = valueNumber
        self.unit = unit
        self.status = status
        self.confidence = confidence
        self.sourceSignal = sourceSignal
        self.rawPayloadRef = rawPayloadRef
    }
}

extension OBDSignalRecord {
    /// Preserve all observation fields while attaching the active session context.
    func withSessionID(_ sessionID: UUID?) -> OBDSignalRecord {
        OBDSignalRecord(
            id: id,
            observedAt: observedAt,
            sessionID: sessionID,
            signalKey: signalKey,
            valueNumber: valueNumber,
            unit: unit,
            status: status,
            confidence: confidence,
            sourceSignal: sourceSignal,
            rawPayloadRef: rawPayloadRef
        )
    }
}
