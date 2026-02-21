import Foundation

/// File-backed queue store that writes to Application Support for durability.
final class FileTelemetryUploadQueueStore: TelemetryUploadQueueStore {
    private let fileURL: URL
    private let fileManager: FileManager
    private let encoder: JSONEncoder
    private let decoder: JSONDecoder

    init(
        fileURL: URL? = nil,
        fileManager: FileManager = .default
    ) {
        self.fileManager = fileManager
        if let fileURL {
            self.fileURL = fileURL
        } else {
            let appSupportDirectory = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
                ?? fileManager.temporaryDirectory
            self.fileURL = appSupportDirectory
                .appendingPathComponent("CarRanks", isDirectory: true)
                .appendingPathComponent("telemetry-upload-queue.json")
        }

        encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        encoder.dateEncodingStrategy = .iso8601

        decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
    }

    func load() throws -> [TelemetryPendingBatch] {
        guard fileManager.fileExists(atPath: fileURL.path) else {
            return []
        }

        let data = try Data(contentsOf: fileURL)
        guard !data.isEmpty else {
            return []
        }
        return try decoder.decode([TelemetryPendingBatch].self, from: data)
    }

    func save(_ queue: [TelemetryPendingBatch]) throws {
        let directoryURL = fileURL.deletingLastPathComponent()
        try fileManager.createDirectory(at: directoryURL, withIntermediateDirectories: true)
        let data = try encoder.encode(queue)
        try data.write(to: fileURL, options: [.atomic])
    }
}
