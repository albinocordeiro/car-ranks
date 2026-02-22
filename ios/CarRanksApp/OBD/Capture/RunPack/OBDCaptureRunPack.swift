import Foundation

/// Upload metadata attached to a run pack so investigators can correlate local traces with backend ingest IDs.
struct OBDRunPackUploadReceipt: Codable, Equatable {
    let batchID: UUID
    let ingestID: UUID
    let accepted: Bool
    let uploadedAt: Date
    let message: String?

    private enum CodingKeys: String, CodingKey {
        case batchID = "batch_id"
        case ingestID = "ingest_id"
        case accepted
        case uploadedAt = "uploaded_at"
        case message
    }
}

/// Portable artifact describing one capture session and its full command-level trace.
struct OBDCaptureRunPack: Identifiable, Codable, Equatable {
    let id: UUID
    let sessionID: UUID
    let userID: UUID
    let vehicleUID: UUID
    let appVersion: String
    let adapterFingerprint: String?
    let adapterIdentitySummary: String?
    let initializationProfileSummary: String?
    let captureWindowStartedAt: Date
    let captureWindowEndedAt: Date
    let sampleIntervalSeconds: Int
    let commandExchanges: [OBDCommandExchange]
    let uploadReceipt: OBDRunPackUploadReceipt?
    let generatedAt: Date

    init(
        sessionID: UUID,
        userID: UUID,
        vehicleUID: UUID,
        appVersion: String,
        adapterFingerprint: String?,
        adapterIdentitySummary: String?,
        initializationProfileSummary: String?,
        captureWindowStartedAt: Date,
        captureWindowEndedAt: Date,
        sampleIntervalSeconds: Int,
        commandExchanges: [OBDCommandExchange],
        uploadReceipt: OBDRunPackUploadReceipt?,
        generatedAt: Date = Date()
    ) {
        id = sessionID
        self.sessionID = sessionID
        self.userID = userID
        self.vehicleUID = vehicleUID
        self.appVersion = appVersion
        self.adapterFingerprint = adapterFingerprint
        self.adapterIdentitySummary = adapterIdentitySummary
        self.initializationProfileSummary = initializationProfileSummary
        self.captureWindowStartedAt = captureWindowStartedAt
        self.captureWindowEndedAt = captureWindowEndedAt
        self.sampleIntervalSeconds = sampleIntervalSeconds
        self.commandExchanges = commandExchanges
        self.uploadReceipt = uploadReceipt
        self.generatedAt = generatedAt
    }

    private enum CodingKeys: String, CodingKey {
        case id
        case sessionID = "session_id"
        case userID = "user_id"
        case vehicleUID = "vehicle_uid"
        case appVersion = "app_version"
        case adapterFingerprint = "adapter_fingerprint"
        case adapterIdentitySummary = "adapter_identity_summary"
        case initializationProfileSummary = "initialization_profile_summary"
        case captureWindowStartedAt = "capture_window_started_at"
        case captureWindowEndedAt = "capture_window_ended_at"
        case sampleIntervalSeconds = "sample_interval_seconds"
        case commandExchanges = "command_exchanges"
        case uploadReceipt = "upload_receipt"
        case generatedAt = "generated_at"
    }
}
