import XCTest
@testable import CarRanksApp

final class OBDDiagnosticSnapshotTests: XCTestCase {
    func testInitNormalizesAndDeduplicatesCodes() {
        let snapshot = OBDDiagnosticSnapshot(
            observedAt: Date(timeIntervalSince1970: 1_700_000_000),
            milOn: true,
            dtcsActive: [" p010a ", "U0123", "u0123", ""]
        )

        XCTAssertEqual(snapshot.dtcsActive, ["P010A", "U0123"])
    }

    func testStateSignatureIncludesMilAndCodes() {
        let snapshot = OBDDiagnosticSnapshot(
            observedAt: Date(timeIntervalSince1970: 1_700_000_000),
            milOn: false,
            dtcsActive: ["P010A"]
        )

        XCTAssertEqual(snapshot.stateSignature, "0|P010A")
    }
}
