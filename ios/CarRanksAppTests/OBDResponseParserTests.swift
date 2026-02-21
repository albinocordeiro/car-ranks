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
}
