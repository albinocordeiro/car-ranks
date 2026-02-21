import Foundation

enum FixtureLoader {
    static func loadFixture(named name: String) throws -> Data {
        guard let url = Bundle.main.url(forResource: name, withExtension: "json", subdirectory: "Fixtures")
            ?? Bundle.main.url(forResource: name, withExtension: "json")
        else {
            throw BackendError.decode("Fixture not found: \(name).json")
        }
        return try Data(contentsOf: url)
    }
}
