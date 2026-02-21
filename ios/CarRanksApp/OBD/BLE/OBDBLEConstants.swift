import Foundation
import CoreBluetooth

enum OBDBLEConstants {
    /// Common BLE UART services used by popular OBD adapters (ELM327-style and Nordic UART).
    static var candidateServiceUUIDs: [CBUUID] {
        [
            CBUUID(string: "FFE0"),
            CBUUID(string: "FFF0"),
            CBUUID(string: "6E400001-B5A3-F393-E0A9-E50E24DCCA9E"),
        ]
    }
}
