import Foundation

struct TelemetryBatchRequest: Codable, Equatable {
    let batchID: UUID
    let schemaVersion: String
    let vehicleUID: UUID
    let source: String
    let client: Client
    let captureWindow: CaptureWindow
    let records: [SignalRecord]
    let sessionEvents: [SessionEvent]
    let diagnostics: [DiagnosticEvent]

    init(
        batchID: UUID,
        schemaVersion: String = "0.2",
        vehicleUID: UUID,
        source: String = "OBD",
        client: Client,
        captureWindow: CaptureWindow,
        records: [SignalRecord],
        sessionEvents: [SessionEvent],
        diagnostics: [DiagnosticEvent]
    ) {
        self.batchID = batchID
        self.schemaVersion = schemaVersion
        self.vehicleUID = vehicleUID
        self.source = source
        self.client = client
        self.captureWindow = captureWindow
        self.records = records
        self.sessionEvents = sessionEvents
        self.diagnostics = diagnostics
    }

    struct Client: Codable, Equatable {
        let platform: String
        let appVersion: String
        let adapterFingerprint: String

        init(platform: String = "ios", appVersion: String, adapterFingerprint: String) {
            self.platform = platform
            self.appVersion = appVersion
            self.adapterFingerprint = adapterFingerprint
        }

        private enum CodingKeys: String, CodingKey {
            case platform
            case appVersion = "app_version"
            case adapterFingerprint = "adapter_fingerprint"
        }
    }

    struct CaptureWindow: Codable, Equatable {
        let startedAt: String
        let endedAt: String
        let sampleIntervalSeconds: Int

        private enum CodingKeys: String, CodingKey {
            case startedAt = "started_at"
            case endedAt = "ended_at"
            case sampleIntervalSeconds = "sample_interval_seconds"
        }
    }

    struct SignalRecord: Codable, Equatable {
        let observedAt: String
        let sessionID: UUID?
        let signalKey: String
        let valueNumber: Double?
        let valueString: String?
        let valueBool: Bool?
        let valueJSON: String?
        let unit: String?
        let status: String
        let confidence: Double?
        let sourceSignal: String?
        let rawPayloadRef: String?

        private enum CodingKeys: String, CodingKey {
            case observedAt = "observed_at"
            case sessionID = "session_id"
            case signalKey = "signal_key"
            case valueNumber = "value_number"
            case valueString = "value_string"
            case valueBool = "value_bool"
            case valueJSON = "value_json"
            case unit
            case status
            case confidence
            case sourceSignal = "source_signal"
            case rawPayloadRef = "raw_payload_ref"
        }
    }

    struct SessionEvent: Codable, Equatable {
        let eventType: String
        let observedAt: String
        let sessionID: UUID
        let rawPayloadRef: String?

        init(
            eventType: String,
            observedAt: String,
            sessionID: UUID,
            rawPayloadRef: String? = nil
        ) {
            self.eventType = eventType
            self.observedAt = observedAt
            self.sessionID = sessionID
            self.rawPayloadRef = rawPayloadRef
        }

        private enum CodingKeys: String, CodingKey {
            case eventType = "event_type"
            case observedAt = "observed_at"
            case sessionID = "session_id"
            case rawPayloadRef = "raw_payload_ref"
        }
    }

    struct DiagnosticEvent: Codable, Equatable {
        let observedAt: String
        let milOn: Bool
        let dtcsActive: [String]

        private enum CodingKeys: String, CodingKey {
            case observedAt = "observed_at"
            case milOn = "mil_on"
            case dtcsActive = "dtcs_active"
        }
    }

    private enum CodingKeys: String, CodingKey {
        case batchID = "batch_id"
        case schemaVersion = "schema_version"
        case vehicleUID = "vehicle_uid"
        case source
        case client
        case captureWindow = "capture_window"
        case records
        case sessionEvents = "session_events"
        case diagnostics
    }
}

struct TelemetryBatchUploadResponse: Decodable, Equatable {
    struct BatchError: Decodable, Equatable {
        let recordIndex: Int?
        let code: String
        let message: String

        private enum CodingKeys: String, CodingKey {
            case recordIndex = "record_index"
            case code
            case message
        }
    }

    let accepted: Bool
    let batchID: UUID
    let ingestID: UUID
    let duplicate: Bool
    let recordsReceived: Int
    let recordsAccepted: Int
    let recordsRejected: Int
    let errors: [BatchError]
    let nextUploadAfterSeconds: Int

    private enum CodingKeys: String, CodingKey {
        case accepted
        case batchID = "batch_id"
        case ingestID = "ingest_id"
        case duplicate
        case recordsReceived = "records_received"
        case recordsAccepted = "records_accepted"
        case recordsRejected = "records_rejected"
        case errors
        case nextUploadAfterSeconds = "next_upload_after_seconds"
    }
}

extension TelemetryBatchRequest.SignalRecord {
    static func from(obdRecord: OBDSignalRecord) -> Self {
        Self(
            observedAt: TelemetryTimestampFormatter.string(from: obdRecord.observedAt),
            sessionID: obdRecord.sessionID,
            signalKey: obdRecord.signalKey,
            valueNumber: obdRecord.valueNumber,
            valueString: nil,
            valueBool: nil,
            valueJSON: nil,
            unit: obdRecord.unit,
            status: obdRecord.status.rawValue,
            confidence: obdRecord.confidence,
            sourceSignal: obdRecord.sourceSignal,
            rawPayloadRef: obdRecord.rawPayloadRef
        )
    }
}

extension TelemetryBatchRequest.DiagnosticEvent {
    static func from(snapshot: OBDDiagnosticSnapshot) -> Self {
        Self(
            observedAt: TelemetryTimestampFormatter.string(from: snapshot.observedAt),
            milOn: snapshot.milOn,
            dtcsActive: snapshot.dtcsActive
        )
    }
}
