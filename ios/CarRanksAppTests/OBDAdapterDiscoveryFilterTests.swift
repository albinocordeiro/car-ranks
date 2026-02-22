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

    func testExcludedPersonalDeviceNameWinsOverCandidateService() {
        XCTAssertFalse(
            OBDAdapterDiscoveryFilter.isLikelyOBDAdapter(
                name: "MacBook Pro",
                advertisedServiceUUIDs: ["6E400001-B5A3-F393-E0A9-E50E24DCCA9E"]
            )
        )
    }

    func testUnnamedAdapterWithoutKnownServiceIsExcluded() {
        XCTAssertFalse(
            OBDAdapterDiscoveryFilter.isLikelyOBDAdapter(
                name: "Unnamed OBD Adapter",
                advertisedServiceUUIDs: []
            )
        )
    }

    func testUnnamedAdapterWithKnownServiceIsIncluded() {
        XCTAssertTrue(
            OBDAdapterDiscoveryFilter.isLikelyOBDAdapter(
                name: "Unnamed OBD Adapter",
                advertisedServiceUUIDs: ["FFE0"]
            )
        )
    }

    func testNonConnectablePeripheralIsExcluded() {
        XCTAssertFalse(
            OBDAdapterDiscoveryFilter.isLikelyOBDAdapter(
                name: "VEEPEAK",
                advertisedServiceUUIDs: [],
                isConnectable: false
            )
        )
    }
}
