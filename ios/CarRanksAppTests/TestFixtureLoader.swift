import Foundation

private final class FixtureBundleSentinel {}

enum TestFixtureLoader {
    static func loadFixture(named name: String) throws -> Data {
        let candidateBundles = [Bundle(for: FixtureBundleSentinel.self), Bundle.main] + Bundle.allBundles + Bundle.allFrameworks

        for bundle in candidateBundles {
            if let url = bundle.url(forResource: name, withExtension: "json") ??
                bundle.url(forResource: name, withExtension: "json", subdirectory: "Fixtures")
            {
                return try Data(contentsOf: url)
            }
        }

        // Last-resort fallback for local runs where bundles are stripped unexpectedly.
        let sourceFixtureURL = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("CarRanksApp/Fixtures/\(name).json")
        if FileManager.default.fileExists(atPath: sourceFixtureURL.path) {
            return try Data(contentsOf: sourceFixtureURL)
        }

        throw NSError(
            domain: "CarRanksAppTests",
            code: 404,
            userInfo: [NSLocalizedDescriptionKey: "Fixture not found: \(name).json"]
        )
    }

    static func decoder() -> JSONDecoder {
        let decoder = JSONDecoder()
        return decoder
    }
}
