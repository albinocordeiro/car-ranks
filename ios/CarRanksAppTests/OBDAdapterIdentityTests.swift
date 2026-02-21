import XCTest
@testable import CarRanksApp

final class OBDAdapterIdentityTests: XCTestCase {
    func testDetectsOBDLinkFamily() {
        let identity = OBDAdapterIdentity.fromATIResponse("OBDLink CX v2.5\r>")
        XCTAssertEqual(identity.profile, .obdLink)
    }

    func testDetectsELMFamily() {
        let identity = OBDAdapterIdentity.fromATIResponse("ELM327 v1.5\r>")
        XCTAssertEqual(identity.profile, .elm327)
    }

    func testFallsBackToGenericFamily() {
        let identity = OBDAdapterIdentity.fromATIResponse("Some Unknown Adapter")
        XCTAssertEqual(identity.profile, .generic)
    }
}
