import XCTest
@testable import CarRanksApp

final class OBDResponseParserTests: XCTestCase {
    func testDecodeSpeedKmh() {
        let raw = "41 0D 28\r>"
        let value = OBDResponseParser.decodeSpeedKmh(rawResponse: raw)
        XCTAssertEqual(value, 40)
    }

    func testDecodeSpeedKmhFromCompactHexToken() {
        let raw = "410D14\r>"
        let value = OBDResponseParser.decodeSpeedKmh(rawResponse: raw)
        XCTAssertEqual(value, 20)
    }

    func testDecodeSpeedKmhIgnoresRepeatedPromptMarkersAndNoise() {
        let raw = "SEARCHING...\r41 0D 2A\r>\r>\r"
        let value = OBDResponseParser.decodeSpeedKmh(rawResponse: raw)
        XCTAssertEqual(value, 42)
    }

    func testDecodeControlModuleVoltage() {
        let raw = "41 42 0D 9A\r>"
        let value = OBDResponseParser.decodeControlModuleVoltage(rawResponse: raw)
        XCTAssertEqual(value ?? 0, 3.482, accuracy: 0.001)
    }

    func testDecodeControlModuleVoltageUsesLastCompleteFrameWhenRepeated() {
        let raw = "41 42 0D 9A 41 42 0D A0\r>"
        let value = OBDResponseParser.decodeControlModuleVoltage(rawResponse: raw)
        XCTAssertEqual(value ?? 0, 3.488, accuracy: 0.001)
    }

    func testDecodeAdapterSupplyVoltageFromATRV() {
        let raw = "ATRV\r12.6V\r>"
        let value = OBDResponseParser.decodeAdapterSupplyVoltage(rawResponse: raw)
        XCTAssertEqual(value ?? 0, 12.6, accuracy: 0.001)
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

    func testDecodeReturnsNilWhenPidPayloadIsPartial() {
        let raw = "41 0D\r>"
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

    func testDecodeSupportedMode1PidsForBlock00() {
        // PID 0x0D (vehicle speed) -> second byte bit 3.
        let raw = "41 00 00 08 00 00\r>"
        let supported = OBDResponseParser.decodeSupportedMode1Pids(
            rawResponse: raw,
            blockBasePID: 0x00
        )

        XCTAssertNotNil(supported)
        XCTAssertTrue(supported?.contains(0x0D) == true)
        XCTAssertFalse(supported?.contains(0x42) == true)
    }

    func testDecodeSupportedMode1PidsForBlock40() {
        // PIDs 0x42 and 0x46 -> first byte bits 6 and 2.
        let raw = "41 40 44 00 00 00\r>"
        let supported = OBDResponseParser.decodeSupportedMode1Pids(
            rawResponse: raw,
            blockBasePID: 0x40
        )

        XCTAssertNotNil(supported)
        XCTAssertTrue(supported?.contains(0x42) == true)
        XCTAssertTrue(supported?.contains(0x46) == true)
    }

    func testDecodeSupportedMode1PidsForCondensedBlock40Token() {
        let raw = "414044000000\r>"
        let supported = OBDResponseParser.decodeSupportedMode1Pids(
            rawResponse: raw,
            blockBasePID: 0x40
        )

        XCTAssertNotNil(supported)
        XCTAssertTrue(supported?.contains(0x42) == true)
        XCTAssertTrue(supported?.contains(0x46) == true)
    }

    func testDecodeSupportedMode1PidsFromFragmentedHexTokens() {
        let raw = "SEARCHING... 41000 008000 0\r>"
        let supported = OBDResponseParser.decodeSupportedMode1Pids(
            rawResponse: raw,
            blockBasePID: 0x00
        )

        XCTAssertNotNil(supported)
        XCTAssertTrue(supported?.contains(0x0D) == true)
    }

    func testDecodeSupportedMode1PidsUnionsMultipleFramesInSameResponse() {
        let raw = "41 40 44 00 00 00 41 40 00 00 00 21\r>"
        let supported = OBDResponseParser.decodeSupportedMode1Pids(
            rawResponse: raw,
            blockBasePID: 0x40
        )

        XCTAssertNotNil(supported)
        XCTAssertTrue(supported?.contains(0x42) == true)
        XCTAssertTrue(supported?.contains(0x46) == true)
        XCTAssertTrue(supported?.contains(0x5B) == true)
        XCTAssertTrue(supported?.contains(0x60) == true)
    }

    func testDecodeSupportedMode1PidsIgnoresNoiseAroundHexPayload() {
        // Noise words should be ignored while compact hex payload is still decoded.
        let raw = "SEARCHING... HIHI 410000080000 NO DATA\r>"
        let supported = OBDResponseParser.decodeSupportedMode1Pids(
            rawResponse: raw,
            blockBasePID: 0x00
        )

        XCTAssertNotNil(supported)
        XCTAssertTrue(supported?.contains(0x0D) == true)
    }

    func testDecodeSupportedMode1PidsReturnsNilWhenPayloadMissing() {
        let raw = "NO DATA\r>"
        XCTAssertNil(
            OBDResponseParser.decodeSupportedMode1Pids(
                rawResponse: raw,
                blockBasePID: 0x40
            )
        )
    }
}
