import XCTest
@testable import CarRanksApp

final class OBDConnectionStateTests: XCTestCase {
    func testReconnectingStatusTextIncludesAttemptProgress() {
        let state = OBDConnectionState.reconnecting(name: "OBD Adapter", attempt: 2, maxAttempts: 4)
        XCTAssertEqual(state.statusText, "Reconnecting to OBD Adapter (2/4)")
    }
}
