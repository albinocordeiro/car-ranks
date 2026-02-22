import Foundation

/// Persisted artifact store for local capture run packs.
///
/// The store lives in Application Support so run packs survive app relaunches and can be exported
/// after a drive without requiring another real-world capture.
final class OBDCaptureRunPackStore {
    private let fileManager: FileManager
    private let encoder: JSONEncoder
    private let decoder: JSONDecoder
    private let directoryURL: URL

    init(
        fileManager: FileManager = .default,
        directoryURL: URL? = nil
    ) {
        self.fileManager = fileManager

        if let directoryURL {
            self.directoryURL = directoryURL
        } else {
            let appSupportDirectory = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
                ?? fileManager.temporaryDirectory
            self.directoryURL = appSupportDirectory
                .appendingPathComponent("CarRanks", isDirectory: true)
                .appendingPathComponent("obd-run-packs", isDirectory: true)
        }

        encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        encoder.dateEncodingStrategy = .iso8601

        decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
    }

    @discardableResult
    func save(_ runPack: OBDCaptureRunPack) throws -> URL {
        try fileManager.createDirectory(at: directoryURL, withIntermediateDirectories: true)
        let fileURL = url(for: runPack.sessionID)
        let encoded = try encoder.encode(runPack)
        try encoded.write(to: fileURL, options: [.atomic])
        return fileURL
    }

    func load(sessionID: UUID) throws -> OBDCaptureRunPack? {
        let fileURL = url(for: sessionID)
        guard fileManager.fileExists(atPath: fileURL.path) else {
            return nil
        }

        let data = try Data(contentsOf: fileURL)
        return try decoder.decode(OBDCaptureRunPack.self, from: data)
    }

    func latestFileURL() -> URL? {
        runPackFileURLs(limit: 1).first
    }

    func loadLatest() throws -> OBDCaptureRunPack? {
        guard let fileURL = latestFileURL() else {
            return nil
        }

        let data = try Data(contentsOf: fileURL)
        return try decoder.decode(OBDCaptureRunPack.self, from: data)
    }

    func runPackFileURLs(limit: Int = 100) -> [URL] {
        guard let enumerator = fileManager.enumerator(
            at: directoryURL,
            includingPropertiesForKeys: [.contentModificationDateKey],
            options: [.skipsHiddenFiles]
        ) else {
            return []
        }

        var files: [URL] = []
        for case let fileURL as URL in enumerator {
            guard fileURL.pathExtension.lowercased() == "json" else {
                continue
            }
            files.append(fileURL)
        }

        files.sort { lhs, rhs in
            let lhsDate = (try? lhs.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate) ?? .distantPast
            let rhsDate = (try? rhs.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate) ?? .distantPast
            return lhsDate > rhsDate
        }

        if limit > 0, files.count > limit {
            return Array(files.prefix(limit))
        }
        return files
    }

    private func url(for sessionID: UUID) -> URL {
        directoryURL.appendingPathComponent("\(sessionID.uuidString.lowercased()).json")
    }
}
