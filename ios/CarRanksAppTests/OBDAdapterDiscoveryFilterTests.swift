import XCTest
@testable import CarRanksApp

final class OBDAdapterDiscoveryFilterTests: XCTestCase {
    func testVeepeakNameIsClassifiedAsLikelyOBD() {
        XCTAssertTrue(
            OBDAdapterDiscoveryFilter.isLikelyOBDAdapter(
                name: "VEEPEAK",
                advertisedServiceUUIDs: []
            )
        )
    }

    func testMacBookIsExcludedFromLikelyOBDResults() {
        XCTAssertFalse(
            OBDAdapterDiscoveryFilter.isLikelyOBDAdapter(
                name: "MacBook Pro",
                advertisedServiceUUIDs: []
            )
        )
    }

    func testKnownOBDServiceUUIDIsClassifiedAsLikelyOBD() {
        XCTAssertTrue(
            OBDAdapterDiscoveryFilter.isLikelyOBDAdapter(
                name: "Some BLE Device",
                advertisedServiceUUIDs: ["6E400001-B5A3-F393-E0A9-E50E24DCCA9E"]
            )
        )
    }
}
