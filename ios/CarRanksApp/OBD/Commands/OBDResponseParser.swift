import Foundation

struct OBDReadinessStatus: Equatable {
    let milOn: Bool
    let storedDTCCount: Int
}

enum OBDResponseParser {
    /// Extract bytes for one `01xx` PID payload from noisy ELM-style responses.
    static func extractPidPayload(rawResponse: String, mode: UInt8, pid: UInt8) -> [UInt8]? {
        extractPidPayload(
            rawResponse: rawResponse,
            mode: mode,
            pid: pid,
            expectedPayloadLength: nil
        )
    }

    /// Extract a fixed-length payload and prefer the last complete frame when multiple are present.
    private static func extractPidPayload(
        rawResponse: String,
        mode: UInt8,
        pid: UInt8,
        expectedPayloadLength: Int?
    ) -> [UInt8]? {
        if let expectedPayloadLength {
            // Some adapters repeat the same PID frame several times in one read; we keep the
            // last complete frame because it is typically the one nearest the prompt terminator.
            return extractFixedLengthPidPayloads(
                rawResponse: rawResponse,
                mode: mode,
                pid: pid,
                expectedPayloadLength: expectedPayloadLength
            ).last
        }

        let responseMode = mode &+ 0x40
        let bytes = extractHexBytes(from: rawResponse)
        guard bytes.count >= 3 else {
            return nil
        }

        var extractedPayload: [UInt8]?
        for index in bytes.indices.dropLast() where bytes[index] == responseMode && bytes[index + 1] == pid {
            let payloadIndex = index + 2
            guard bytes.indices.contains(payloadIndex) else {
                continue
            }
            extractedPayload = Array(bytes[payloadIndex...])
        }
        return extractedPayload
    }

    static func decodeSpeedKmh(rawResponse: String) -> Double? {
        guard let payload = extractPidPayload(
            rawResponse: rawResponse,
            mode: 0x01,
            pid: 0x0D,
            expectedPayloadLength: 1
        ),
              let first = payload.first
        else {
            return nil
        }
        return Double(first)
    }

    static func decodeControlModuleVoltage(rawResponse: String) -> Double? {
        guard let payload = extractPidPayload(
            rawResponse: rawResponse,
            mode: 0x01,
            pid: 0x42,
            expectedPayloadLength: 2
        ),
              payload.count == 2
        else {
            return nil
        }
        return Double(Int(payload[0]) * 256 + Int(payload[1])) / 1000.0
    }

    static func decodeAmbientTemperatureC(rawResponse: String) -> Double? {
        guard let payload = extractPidPayload(
            rawResponse: rawResponse,
            mode: 0x01,
            pid: 0x46,
            expectedPayloadLength: 1
        ),
              let first = payload.first
        else {
            return nil
        }
        return Double(Int(first) - 40)
    }

    /// Parse adapter-reported supply voltage from `ATRV` responses like `12.6V` or `12.6`.
    static func decodeAdapterSupplyVoltage(rawResponse: String) -> Double? {
        let separators = CharacterSet.whitespacesAndNewlines
            .union(CharacterSet(charactersIn: "\r\n>,;|"))
        let tokens = rawResponse
            .uppercased()
            .components(separatedBy: separators)
            .map { $0.trimmingCharacters(in: CharacterSet(charactersIn: ".:")) }
            .filter { !$0.isEmpty }

        // We read from the end because many adapters echo `ATRV` first and append the value later.
        for token in tokens.reversed() {
            let numericCandidate = token.trimmingCharacters(in: CharacterSet(charactersIn: "V"))
            guard let voltage = Double(numericCandidate) else {
                continue
            }
            if (6.0...30.0).contains(voltage) {
                return voltage
            }
        }
        return nil
    }

    static func decodeReadinessStatus(rawResponse: String) -> OBDReadinessStatus? {
        guard let payload = extractPidPayload(
            rawResponse: rawResponse,
            mode: 0x01,
            pid: 0x01,
            expectedPayloadLength: 4
        ),
              let first = payload.first
        else {
            return nil
        }

        let milOn = (first & 0x80) != 0
        let storedDTCCount = Int(first & 0x7F)
        return OBDReadinessStatus(milOn: milOn, storedDTCCount: storedDTCCount)
    }

    /// Decode a Mode 01 support bitmask response (`0100`, `0120`, `0140`, ...).
    static func decodeSupportedMode1Pids(
        rawResponse: String,
        blockBasePID: UInt8
    ) -> Set<UInt8>? {
        guard blockBasePID % 0x20 == 0 else {
            return nil
        }

        let payloads = extractFixedLengthPidPayloads(
            rawResponse: rawResponse,
            mode: 0x01,
            pid: blockBasePID,
            expectedPayloadLength: 4
        )
        guard !payloads.isEmpty
        else {
            return nil
        }

        var supportedPids: Set<UInt8> = []
        for payload in payloads {
            for (byteIndex, byteValue) in payload.prefix(4).enumerated() {
                for bitOffset in 0..<8 {
                    let bitMask: UInt8 = 1 << (7 - bitOffset)
                    guard (byteValue & bitMask) != 0 else {
                        continue
                    }

                    let pidValue = Int(blockBasePID) + (byteIndex * 8) + bitOffset + 1
                    guard (1...255).contains(pidValue) else {
                        continue
                    }
                    supportedPids.insert(UInt8(pidValue))
                }
            }
        }

        return supportedPids
    }

    /// Decode stored trouble codes from a Mode 03 response (`43`).
    static func decodeStoredDiagnosticTroubleCodes(rawResponse: String) -> [String] {
        let bytes = extractHexBytes(from: rawResponse)
        guard let responseIndex = bytes.firstIndex(of: 0x43) else {
            return []
        }

        var decoded: [String] = []
        var seen: Set<String> = []
        var index = responseIndex + 1
        while bytes.indices.contains(index + 1) {
            let msb = bytes[index]
            let lsb = bytes[index + 1]
            if let code = decodeDiagnosticTroubleCode(msb: msb, lsb: lsb),
               seen.insert(code).inserted
            {
                decoded.append(code)
            }
            index += 2
        }
        return decoded
    }

    private static func extractHexBytes(from rawResponse: String) -> [UInt8] {
        // Many adapters emit compact frames (for example `414000000021`) and interleave
        // plain-text noise (`SEARCHING...`, `NO DATA`, adapter banners). We keep only
        // tokens that are fully hex and split long tokens into byte pairs.
        //
        // Some firmware variants fragment one frame across adjacent hex-only chunks
        // (`410000 000 000`). We stitch adjacent hex tokens into one run before parsing.
        let uppercased = rawResponse.uppercased()
        let separators = CharacterSet.whitespacesAndNewlines
            .union(CharacterSet(charactersIn: "\r\n>,;|"))
        let tokens = uppercased.components(separatedBy: separators)
            .map { $0.trimmingCharacters(in: CharacterSet(charactersIn: ".:")) }
            .filter { !$0.isEmpty }

        var hexRuns: [String] = []
        var currentRun = ""
        for token in tokens {
            if token.allSatisfy(\.isHexDigit) {
                currentRun.append(token)
            } else if !currentRun.isEmpty {
                hexRuns.append(currentRun)
                currentRun = ""
            }
        }
        if !currentRun.isEmpty {
            hexRuns.append(currentRun)
        }

        return hexRuns.flatMap { run -> [UInt8] in
            let evenCharacterCount = run.count - (run.count % 2)
            guard evenCharacterCount >= 2 else {
                return []
            }

            var parsed: [UInt8] = []
            var runIndex = run.startIndex
            let endIndex = run.index(run.startIndex, offsetBy: evenCharacterCount)
            while runIndex < endIndex {
                let nextIndex = run.index(runIndex, offsetBy: 2)
                let hexPair = run[runIndex..<nextIndex]
                if let value = UInt8(hexPair, radix: 16) {
                    parsed.append(value)
                }
                runIndex = nextIndex
            }
            return parsed
        }
    }

    private static func extractFixedLengthPidPayloads(
        rawResponse: String,
        mode: UInt8,
        pid: UInt8,
        expectedPayloadLength: Int
    ) -> [[UInt8]] {
        let responseMode = mode &+ 0x40
        let bytes = extractHexBytes(from: rawResponse)
        guard bytes.count >= expectedPayloadLength + 2 else {
            return []
        }

        var payloads: [[UInt8]] = []
        let lastStartIndex = bytes.count - (expectedPayloadLength + 2)
        for index in 0...lastStartIndex where bytes[index] == responseMode && bytes[index + 1] == pid {
            let payloadStartIndex = index + 2
            let payloadEndExclusive = payloadStartIndex + expectedPayloadLength
            payloads.append(Array(bytes[payloadStartIndex..<payloadEndExclusive]))
        }
        return payloads
    }

    private static func decodeDiagnosticTroubleCode(msb: UInt8, lsb: UInt8) -> String? {
        guard !(msb == 0x00 && lsb == 0x00) else {
            return nil
        }

        let prefix: String
        switch (msb & 0xC0) >> 6 {
        case 0:
            prefix = "P"
        case 1:
            prefix = "C"
        case 2:
            prefix = "B"
        default:
            prefix = "U"
        }

        let digit1 = String((msb & 0x30) >> 4)
        let digit2 = nibbleHexString(msb & 0x0F)
        let digit3 = nibbleHexString((lsb & 0xF0) >> 4)
        let digit4 = nibbleHexString(lsb & 0x0F)
        return "\(prefix)\(digit1)\(digit2)\(digit3)\(digit4)"
    }

    private static func nibbleHexString(_ value: UInt8) -> String {
        String(value, radix: 16, uppercase: true)
    }
}
