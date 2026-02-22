import Foundation

/// Canonical parse outcomes used when serializing command-level traces for investigation packs.
enum OBDCommandParseOutcome: String, Codable, Equatable {
    case ok
    case unavailable
    case error
    case notSupported = "not_supported"
}

/// One command/response transaction captured during a drive session.
///
/// This is intentionally verbose because it becomes the source artifact for fixture generation,
/// parser debugging, and adapter compatibility investigations.
struct OBDCommandExchange: Identifiable, Codable, Equatable {
    let id: UUID
    let startedAt: Date
    let endedAt: Date
    let command: String
    let rawResponse: String?
    let errorMessage: String?
    let parseOutcome: OBDCommandParseOutcome
    let signalKey: String?
    let sourceSignal: String?

    init(
        id: UUID = UUID(),
        startedAt: Date,
        endedAt: Date,
        command: String,
        rawResponse: String?,
        errorMessage: String?,
        parseOutcome: OBDCommandParseOutcome,
        signalKey: String?,
        sourceSignal: String?
    ) {
        self.id = id
        self.startedAt = startedAt
        self.endedAt = endedAt
        self.command = command
        self.rawResponse = rawResponse
        self.errorMessage = errorMessage
        self.parseOutcome = parseOutcome
        self.signalKey = signalKey
        self.sourceSignal = sourceSignal
    }

    private enum CodingKeys: String, CodingKey {
        case id
        case startedAt = "started_at"
        case endedAt = "ended_at"
        case command
        case rawResponse = "raw_response"
        case errorMessage = "error_message"
        case parseOutcome = "parse_outcome"
        case signalKey = "signal_key"
        case sourceSignal = "source_signal"
    }
}
