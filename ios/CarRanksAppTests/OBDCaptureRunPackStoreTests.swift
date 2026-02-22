import XCTest
@testable import CarRanksApp

final class OBDCaptureRunPackStoreTests: XCTestCase {
    func testSaveAndLoadRoundTripWithLargeTranscript() throws {
        let directoryURL = makeTemporaryDirectoryURL()
        defer { try? FileManager.default.removeItem(at: directoryURL) }

        let store = OBDCaptureRunPackStore(directoryURL: directoryURL)
        let runPack = makeRunPack(
            sessionID: UUID(uuidString: "7C9D2D53-2D50-4C9F-AF84-6C1DB02F9F7C")!,
            commandCount: 320,
            generatedAt: Date(timeIntervalSince1970: 1_770_000_000)
        )

        let saveURL = try store.save(runPack)
        XCTAssertTrue(FileManager.default.fileExists(atPath: saveURL.path))

        let loaded = try XCTUnwrap(store.load(sessionID: runPack.sessionID))
        XCTAssertEqual(loaded, runPack)
        XCTAssertEqual(loaded.commandExchanges.count, 320)
    }

    func testLoadLatestReturnsMostRecentRunPack() throws {
        let directoryURL = makeTemporaryDirectoryURL()
        defer { try? FileManager.default.removeItem(at: directoryURL) }

        let store = OBDCaptureRunPackStore(directoryURL: directoryURL)
        let first = makeRunPack(
            sessionID: UUID(uuidString: "E1C4017C-1CA4-4C98-A503-5AD9091014A0")!,
            commandCount: 2,
            generatedAt: Date(timeIntervalSince1970: 1_770_000_100)
        )
        let second = makeRunPack(
            sessionID: UUID(uuidString: "13B1BD37-7A9A-49D5-96E7-2076257AB576")!,
            commandCount: 3,
            generatedAt: Date(timeIntervalSince1970: 1_770_000_200)
        )

        _ = try store.save(first)
        Thread.sleep(forTimeInterval: 0.02)
        _ = try store.save(second)

        let latest = try XCTUnwrap(store.loadLatest())
        XCTAssertEqual(latest.sessionID, second.sessionID)
    }

    func testLoadReturnsNilWhenRunPackIsMissing() throws {
        let directoryURL = makeTemporaryDirectoryURL()
        defer { try? FileManager.default.removeItem(at: directoryURL) }

        let store = OBDCaptureRunPackStore(directoryURL: directoryURL)
        let missing = try store.load(sessionID: UUID())
        XCTAssertNil(missing)
    }

    private func makeTemporaryDirectoryURL() -> URL {
        let directoryURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("car-ranks-run-pack-tests", isDirectory: true)
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try? FileManager.default.createDirectory(at: directoryURL, withIntermediateDirectories: true)
        return directoryURL
    }

    private func makeRunPack(
        sessionID: UUID,
        commandCount: Int,
        generatedAt: Date
    ) -> OBDCaptureRunPack {
        let startedAt = Date(timeIntervalSince1970: 1_770_000_000)
        let endedAt = Date(timeIntervalSince1970: 1_770_000_420)
        let uploadAt = Date(timeIntervalSince1970: 1_770_000_500)
        let commandExchanges = (0 ..< commandCount).map { index in
            OBDCommandExchange(
                id: UUID(uuidString: String(format: "00000000-0000-0000-0000-%012d", index + 1))!,
                startedAt: startedAt.addingTimeInterval(Double(index)),
                endedAt: startedAt.addingTimeInterval(Double(index) + 1),
                command: String(format: "01%02X", index % 256),
                rawResponse: "41 0D \(index) \(String(repeating: "AA", count: 16))",
                errorMessage: nil,
                parseOutcome: .ok,
                signalKey: "speed.vehicle",
                sourceSignal: "01_0D"
            )
        }

        return OBDCaptureRunPack(
            sessionID: sessionID,
            userID: UUID(uuidString: "15D35C69-4BB8-403A-B321-DB7B7A37D7FA")!,
            vehicleUID: UUID(uuidString: "E11889BF-504C-4238-9583-BC8840F20E19")!,
            appVersion: "1.0.0",
            adapterFingerprint: "adapter-1",
            adapterIdentitySummary: "ELM327 v1.5",
            initializationProfileSummary: "ELM327",
            captureWindowStartedAt: startedAt,
            captureWindowEndedAt: endedAt,
            sampleIntervalSeconds: 5,
            commandExchanges: commandExchanges,
            uploadReceipt: OBDRunPackUploadReceipt(
                batchID: UUID(uuidString: "AB8F5A3F-C471-4ED8-9F80-4A04C53A9168")!,
                ingestID: UUID(uuidString: "6D4EBE86-AE87-44F2-8B88-BFC4F2A5EA88")!,
                accepted: true,
                uploadedAt: uploadAt,
                message: nil
            ),
            generatedAt: generatedAt
        )
    }
}
