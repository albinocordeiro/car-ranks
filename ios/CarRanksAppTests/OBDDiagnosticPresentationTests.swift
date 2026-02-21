import XCTest
@testable import CarRanksApp

final class OBDDiagnosticPresentationTests: XCTestCase {
    func testFromWithoutSnapshotUsesUnknownDefaults() {
        let presentation = OBDDiagnosticPresentation.from(
            latestSnapshot: nil,
            lastChangedAt: nil
        )

        XCTAssertNil(presentation.milIsOn)
        XCTAssertEqual(presentation.milStatusText, "Unknown")
        XCTAssertEqual(presentation.activeDTCsSummary, "None")
        XCTAssertEqual(presentation.lastObservedText, "Not observed yet")
        XCTAssertEqual(presentation.changeMarkerText, "No diagnostic state changes recorded yet.")
        XCTAssertFalse(presentation.isChangeMarkerHighlighted)
    }

    func testFromSnapshotMapsMilAndDTCSummary() {
        let observedAt = Date(timeIntervalSince1970: 1_700_000_100)
        let snapshot = OBDDiagnosticSnapshot(
            observedAt: observedAt,
            milOn: true,
            dtcsActive: ["P010A", "U0123"]
        )

        let presentation = OBDDiagnosticPresentation.from(
            latestSnapshot: snapshot,
            lastChangedAt: nil
        )

        XCTAssertEqual(presentation.milIsOn, true)
        XCTAssertEqual(presentation.milStatusText, "On")
        XCTAssertEqual(presentation.activeDTCs, ["P010A", "U0123"])
        XCTAssertEqual(presentation.activeDTCsSummary, "2 active")
        XCTAssertEqual(
            presentation.lastObservedText,
            TelemetryTimestampFormatter.string(from: observedAt)
        )
        XCTAssertFalse(presentation.isChangeMarkerHighlighted)
    }

    func testFromSnapshotWithChangeMarkerHighlightsChange() {
        let observedAt = Date(timeIntervalSince1970: 1_700_000_100)
        let changedAt = Date(timeIntervalSince1970: 1_700_000_130)
        let snapshot = OBDDiagnosticSnapshot(
            observedAt: observedAt,
            milOn: false,
            dtcsActive: []
        )

        let presentation = OBDDiagnosticPresentation.from(
            latestSnapshot: snapshot,
            lastChangedAt: changedAt
        )

        XCTAssertEqual(presentation.milStatusText, "Off")
        XCTAssertEqual(presentation.activeDTCsSummary, "None")
        XCTAssertTrue(presentation.isChangeMarkerHighlighted)
        XCTAssertEqual(
            presentation.changeMarkerText,
            "Diagnostic state changed at \(TelemetryTimestampFormatter.string(from: changedAt))"
        )
    }
}
