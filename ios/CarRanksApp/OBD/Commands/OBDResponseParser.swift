import Foundation

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
}
