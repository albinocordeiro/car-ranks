import XCTest
@testable import CarRanksApp

final class OBDResponseParserTests: XCTestCase {
    func testDecodeSpeedKmh() {
        let raw = "41 0D 28\r>"
        let value = OBDResponseParser.decodeSpeedKmh(rawResponse: raw)
        XCTAssertEqual(value, 40)
    }

    func testDecodeControlModuleVoltage() {
        let raw = "41 42 0D 9A\r>"
        let value = OBDResponseParser.decodeControlModuleVoltage(rawResponse: raw)
        XCTAssertEqual(value ?? 0, 3.482, accuracy: 0.001)
    }

    func testDecodeAmbientTemperature() {
        let raw = "41 46 22\r>"
        let value = OBDResponseParser.decodeAmbientTemperatureC(rawResponse: raw)
        XCTAssertEqual(value, -6)
    }

    func testDecodeReturnsNilWhenPidPayloadIsMissing() {
        let raw = "NO DATA\r>"
        XCTAssertNil(OBDResponseParser.decodeSpeedKmh(rawResponse: raw))
    }

    func testDecodeReadinessStatus() {
        let raw = "41 01 82 00 00 00\r>"
        let status = OBDResponseParser.decodeReadinessStatus(rawResponse: raw)
        XCTAssertEqual(status?.milOn, true)
        XCTAssertEqual(status?.storedDTCCount, 2)
    }

    func testDecodeStoredDiagnosticTroubleCodes() {
        let raw = "43 01 0A C1 23 00 00\r>"
        let codes = OBDResponseParser.decodeStoredDiagnosticTroubleCodes(rawResponse: raw)
        XCTAssertEqual(codes, ["P010A", "U0123"])
    }

    func testDecodeStoredDiagnosticTroubleCodesReturnsEmptyWhenMissingModeHeader() {
        let raw = "NO DATA\r>"
        XCTAssertTrue(OBDResponseParser.decodeStoredDiagnosticTroubleCodes(rawResponse: raw).isEmpty)
    }
}
