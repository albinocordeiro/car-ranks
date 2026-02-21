import XCTest
@preconcurrency import CoreBluetooth
@testable import CarRanksApp

final class OBDBluetoothStateMessageTests: XCTestCase {
    func testPoweredOnReturnsNilMessage() {
        XCTAssertNil(OBDBluetoothStateMessage.forState(.poweredOn))
    }

    func testUnauthorizedIncludesSettingsGuidance() {
        let message = OBDBluetoothStateMessage.forState(.unauthorized)
        XCTAssertEqual(
            message,
            "Bluetooth access is denied. Enable it in Settings > Privacy & Security > Bluetooth."
        )
    }

    func testPoweredOffUsesActionableMessage() {
        let message = OBDBluetoothStateMessage.forState(.poweredOff)
        XCTAssertEqual(message, "Bluetooth is turned off. Turn it on and try again.")
    }
}
