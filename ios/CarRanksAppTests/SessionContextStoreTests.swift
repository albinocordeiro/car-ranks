import Foundation
import XCTest
@testable import CarRanksApp

final class SessionContextStoreTests: XCTestCase {
    func testLoadUsesFallbackValuesWhenStoreIsEmpty() {
        let isolated = makeIsolatedDefaults()
        defer { clear(defaults: isolated.defaults, suiteName: isolated.suiteName) }

        let store = SessionContextStore(defaults: isolated.defaults)
        let fallbackUser = UUID(uuidString: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa")!
        let fallbackVehicle = UUID(uuidString: "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb")!

        let loaded = store.load(defaultUserID: fallbackUser, defaultVehicleUID: fallbackVehicle)
        XCTAssertEqual(loaded.session.userID, fallbackUser)
        XCTAssertEqual(loaded.session.vehicleUID, fallbackVehicle)
        XCTAssertEqual(loaded.mode, .mock)
    }

    func testSaveAndLoadRoundTrip() {
        let isolated = makeIsolatedDefaults()
        defer { clear(defaults: isolated.defaults, suiteName: isolated.suiteName) }

        let store = SessionContextStore(defaults: isolated.defaults)
        let savedSession = SessionContext(
            userID: UUID(uuidString: "cccccccc-cccc-cccc-cccc-cccccccccccc")!,
            vehicleUID: UUID(uuidString: "dddddddd-dddd-dddd-dddd-dddddddddddd")!
        )

        store.save(savedSession, mode: .live)
        let loaded = store.load(defaultUserID: UUID(), defaultVehicleUID: UUID())

        XCTAssertEqual(loaded.session, savedSession)
        XCTAssertEqual(loaded.mode, .live)
    }

    func testLoadFallsBackToMockWhenStoredModeIsInvalid() {
        let isolated = makeIsolatedDefaults()
        defer { clear(defaults: isolated.defaults, suiteName: isolated.suiteName) }

        isolated.defaults.set("unknown-mode", forKey: "data_source_mode")
        let store = SessionContextStore(defaults: isolated.defaults)
        let loaded = store.load(defaultUserID: UUID(), defaultVehicleUID: UUID())

        XCTAssertEqual(loaded.mode, .mock)
    }

    private func makeIsolatedDefaults() -> (defaults: UserDefaults, suiteName: String) {
        let suiteName = "CarRanksAppTests.\(UUID().uuidString)"
        return (UserDefaults(suiteName: suiteName)!, suiteName)
    }

    private func clear(defaults: UserDefaults, suiteName: String) {
        defaults.removePersistentDomain(forName: suiteName)
    }
}
