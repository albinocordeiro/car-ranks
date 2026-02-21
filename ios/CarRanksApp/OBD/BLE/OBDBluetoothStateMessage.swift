import Foundation
@preconcurrency import CoreBluetooth

/// User-facing descriptions for CoreBluetooth manager states during adapter discovery.
enum OBDBluetoothStateMessage {
    static func forState(_ state: CBManagerState) -> String? {
        switch state {
        case .poweredOn:
            return nil
        case .poweredOff:
            return "Bluetooth is turned off. Turn it on and try again."
        case .unauthorized:
            return "Bluetooth access is denied. Enable it in Settings > Privacy & Security > Bluetooth."
        case .unsupported:
            return "Bluetooth LE is unavailable on this device."
        case .resetting:
            return "Bluetooth is resetting. Please try again."
        case .unknown:
            return "Bluetooth is still initializing. Please try again in a moment."
        @unknown default:
            return "Bluetooth is unavailable."
        }
    }
}
