import Foundation

struct OBDReadinessStatus: Equatable {
    let milOn: Bool
    let storedDTCCount: Int
}

enum OBDResponseParser {
    /// Extract bytes for one `01xx` PID payload from noisy ELM-style responses.
    static func extractPidPayload(rawResponse: String, mode: UInt8, pid: UInt8) -> [UInt8]? {
        let responseMode = mode &+ 0x40
        let bytes = extractHexBytes(from: rawResponse)

        guard bytes.count >= 3 else {
            return nil
        }

        for index in bytes.indices {
            guard bytes.indices.contains(index + 1) else { continue }
            if bytes[index] == responseMode, bytes[index + 1] == pid {
                let payloadIndex = index + 2
                guard bytes.indices.contains(payloadIndex) else {
                    return nil
                }
                return Array(bytes[payloadIndex...])
            }
        }

        return nil
    }

    static func decodeSpeedKmh(rawResponse: String) -> Double? {
        guard let payload = extractPidPayload(rawResponse: rawResponse, mode: 0x01, pid: 0x0D),
              let first = payload.first
        else {
            return nil
        }
        return Double(first)
    }

    static func decodeControlModuleVoltage(rawResponse: String) -> Double? {
        guard let payload = extractPidPayload(rawResponse: rawResponse, mode: 0x01, pid: 0x42),
              payload.count >= 2
        else {
            return nil
        }
        return Double(Int(payload[0]) * 256 + Int(payload[1])) / 1000.0
    }

    static func decodeAmbientTemperatureC(rawResponse: String) -> Double? {
        guard let payload = extractPidPayload(rawResponse: rawResponse, mode: 0x01, pid: 0x46),
              let first = payload.first
        else {
            return nil
        }
        return Double(Int(first) - 40)
    }

    static func decodeReadinessStatus(rawResponse: String) -> OBDReadinessStatus? {
        guard let payload = extractPidPayload(rawResponse: rawResponse, mode: 0x01, pid: 0x01),
              let first = payload.first
        else {
            return nil
        }

        let milOn = (first & 0x80) != 0
        let storedDTCCount = Int(first & 0x7F)
        return OBDReadinessStatus(milOn: milOn, storedDTCCount: storedDTCCount)
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
        let uppercased = rawResponse.uppercased()
        let separators = CharacterSet.whitespacesAndNewlines.union(CharacterSet(charactersIn: "\r\n>"))
        let tokens = uppercased.components(separatedBy: separators)
            .filter { !$0.isEmpty }

        return tokens.compactMap { token -> UInt8? in
            let normalizedToken = token
                .trimmingCharacters(in: .whitespacesAndNewlines)
                .replacingOccurrences(of: "SEARCHING...", with: "")
                .replacingOccurrences(of: "NO", with: "")
                .replacingOccurrences(of: "DATA", with: "")

            guard normalizedToken.count <= 2,
                  normalizedToken.allSatisfy(\.isHexDigit),
                  let value = UInt8(normalizedToken, radix: 16)
            else {
                return nil
            }
            return value
        }
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
