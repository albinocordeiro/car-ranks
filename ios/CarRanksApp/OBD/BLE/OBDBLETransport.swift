import Foundation

/// Minimal transport contract consumed by command polling/capture logic.
@MainActor
protocol OBDBLETransport: AnyObject {
    var discoveredDevices: [OBDAdapterDevice] { get }
    var connectionState: OBDConnectionState { get }
    var adapterFingerprint: String? { get }

    func startScanning()
    func stopScanning()
    func connect(to deviceID: UUID)
    func disconnect()
    func sendRawCommand(_ command: String) async throws -> String
}
