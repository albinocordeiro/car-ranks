import Foundation
@preconcurrency import CoreBluetooth
import CryptoKit

/// CoreBluetooth-backed adapter client that handles scan/connect and line-based OBD command roundtrips.
@MainActor
final class CoreBluetoothOBDClient: NSObject, ObservableObject, OBDBLETransport {
    @Published private(set) var discoveredDevices: [OBDAdapterDevice] = []
    @Published private(set) var connectionState: OBDConnectionState = .disconnected
    @Published private(set) var adapterFingerprint: String?

    private var centralManager: CBCentralManager!
    private var discoveredPeripheralByID: [UUID: CBPeripheral] = [:]
    private var connectedPeripheral: CBPeripheral?
    private var writeCharacteristic: CBCharacteristic?
    private var notifyCharacteristic: CBCharacteristic?

    private var pendingCommandContinuation: CheckedContinuation<String, Error>?
    private var pendingCommandBuffer = ""
    private var pendingCommandTimeoutTask: Task<Void, Never>?
    private let commandTimeoutNanoseconds: UInt64

    override init() {
        commandTimeoutNanoseconds = 5_000_000_000
        super.init()
        centralManager = CBCentralManager(delegate: self, queue: nil)
    }

    init(commandTimeoutNanoseconds: UInt64) {
        self.commandTimeoutNanoseconds = commandTimeoutNanoseconds
        super.init()
        centralManager = CBCentralManager(delegate: self, queue: nil)
    }

    func startScanning() {
        guard centralManager.state == .poweredOn else {
            connectionState = .error("Bluetooth is unavailable.")
            return
        }

        // We intentionally scan broadly because OBD vendors often use custom service UUIDs.
        discoveredDevices = []
        discoveredPeripheralByID = [:]
        centralManager.scanForPeripherals(
            withServices: nil,
            options: [CBCentralManagerScanOptionAllowDuplicatesKey: false]
        )
        connectionState = .scanning
    }

    func stopScanning() {
        centralManager.stopScan()
        if case .scanning = connectionState {
            connectionState = .disconnected
        }
    }

    func connect(to deviceID: UUID) {
        guard let peripheral = discoveredPeripheralByID[deviceID] else {
            connectionState = .error("Selected adapter is no longer available.")
            return
        }

        stopScanning()
        connectionState = .connecting(peripheral.readableName)
        centralManager.connect(peripheral, options: nil)
    }

    func disconnect() {
        guard let connectedPeripheral else {
            connectionState = .disconnected
            return
        }

        centralManager.cancelPeripheralConnection(connectedPeripheral)
    }

    func sendRawCommand(_ command: String) async throws -> String {
        guard connectionState.isConnected,
              let peripheral = connectedPeripheral,
              let writeCharacteristic
        else {
            throw BackendError.transport("OBD adapter is not connected.")
        }

        guard pendingCommandContinuation == nil else {
            throw BackendError.transport("OBD adapter is busy processing another command.")
        }

        let trimmedCommand = command.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedCommand.isEmpty else {
            throw BackendError.transport("Command cannot be empty.")
        }

        guard let commandData = "\(trimmedCommand)\r".data(using: .utf8) else {
            throw BackendError.transport("Failed to encode command.")
        }

        let writeType: CBCharacteristicWriteType = writeCharacteristic.properties.contains(.write) ? .withResponse : .withoutResponse

        return try await withCheckedThrowingContinuation { continuation in
            pendingCommandContinuation = continuation
            pendingCommandBuffer = ""
            pendingCommandTimeoutTask?.cancel()
            pendingCommandTimeoutTask = Task { [weak self] in
                guard let self else { return }
                try? await Task.sleep(nanoseconds: self.commandTimeoutNanoseconds)
                await MainActor.run {
                    self.finishPendingCommand(with: .failure(BackendError.transport("OBD adapter timeout while waiting for response.")))
                }
            }
            peripheral.writeValue(commandData, for: writeCharacteristic, type: writeType)
        }
    }

    private func updateDiscoveredDevice(
        peripheral: CBPeripheral,
        advertisedServiceUUIDs: [String],
        rssi: Int
    ) {
        discoveredPeripheralByID[peripheral.identifier] = peripheral

        let device = OBDAdapterDevice(
            id: peripheral.identifier,
            name: peripheral.readableName,
            rssi: rssi,
            advertisedServices: advertisedServiceUUIDs,
            lastSeenAt: Date()
        )

        discoveredDevices.removeAll { $0.id == device.id }
        discoveredDevices.append(device)
        discoveredDevices.sort { lhs, rhs in
            if lhs.rssi == rhs.rssi {
                return lhs.name.localizedCaseInsensitiveCompare(rhs.name) == .orderedAscending
            }
            return lhs.rssi > rhs.rssi
        }
    }

    private func refreshIOCharacteristics(for peripheral: CBPeripheral) {
        let services = peripheral.services ?? []
        let prioritizedServices = services.sorted { lhs, rhs in
            let lhsPriority = OBDBLEConstants.candidateServiceUUIDs.contains(lhs.uuid) ? 0 : 1
            let rhsPriority = OBDBLEConstants.candidateServiceUUIDs.contains(rhs.uuid) ? 0 : 1
            return lhsPriority < rhsPriority
        }

        let allCharacteristics = prioritizedServices.flatMap { $0.characteristics ?? [] }
        writeCharacteristic = allCharacteristics.first(where: { characteristic in
            characteristic.properties.contains(.write) || characteristic.properties.contains(.writeWithoutResponse)
        })
        notifyCharacteristic = allCharacteristics.first(where: { characteristic in
            characteristic.properties.contains(.notify) || characteristic.properties.contains(.indicate)
        })

        if let notifyCharacteristic {
            peripheral.setNotifyValue(true, for: notifyCharacteristic)
        }

        if writeCharacteristic == nil || notifyCharacteristic == nil {
            connectionState = .error("Connected adapter does not expose expected BLE UART characteristics.")
            return
        }

        connectionState = .connected(peripheral.readableName)
    }

    private func finishPendingCommand(with result: Result<String, Error>) {
        pendingCommandTimeoutTask?.cancel()
        pendingCommandTimeoutTask = nil

        guard let continuation = pendingCommandContinuation else {
            return
        }

        pendingCommandContinuation = nil
        let rawOutput = pendingCommandBuffer
        pendingCommandBuffer = ""

        switch result {
        case .success:
            continuation.resume(returning: rawOutput)
        case let .failure(error):
            continuation.resume(throwing: error)
        }
    }
}

extension CoreBluetoothOBDClient: CBCentralManagerDelegate {
    nonisolated func centralManagerDidUpdateState(_ central: CBCentralManager) {
        Task { @MainActor in
            if central.state != .poweredOn {
                connectionState = .error("Bluetooth is not powered on.")
                stopScanning()
            }
        }
    }

    nonisolated func centralManager(
        _: CBCentralManager,
        didDiscover peripheral: CBPeripheral,
        advertisementData: [String: Any],
        rssi RSSI: NSNumber
    ) {
        let serviceUUIDs = (advertisementData[CBAdvertisementDataServiceUUIDsKey] as? [CBUUID] ?? [])
            .map(\.uuidString)
            .sorted()
        let rssiValue = RSSI.intValue

        Task { @MainActor in
            updateDiscoveredDevice(
                peripheral: peripheral,
                advertisedServiceUUIDs: serviceUUIDs,
                rssi: rssiValue
            )
        }
    }

    nonisolated func centralManager(_: CBCentralManager, didConnect peripheral: CBPeripheral) {
        Task { @MainActor in
            connectedPeripheral = peripheral
            peripheral.delegate = self
            writeCharacteristic = nil
            notifyCharacteristic = nil
            adapterFingerprint = SHA256
                .hash(data: Data(peripheral.identifier.uuidString.lowercased().utf8))
                .hexString
            connectionState = .connecting(peripheral.readableName)
            peripheral.discoverServices(nil)
        }
    }

    nonisolated func centralManager(_: CBCentralManager, didFailToConnect peripheral: CBPeripheral, error: Error?) {
        Task { @MainActor in
            connectedPeripheral = nil
            writeCharacteristic = nil
            notifyCharacteristic = nil
            let message = error?.localizedDescription ?? "Unknown connection failure."
            connectionState = .error("Failed to connect to \(peripheral.readableName): \(message)")
        }
    }

    nonisolated func centralManager(
        _: CBCentralManager,
        didDisconnectPeripheral peripheral: CBPeripheral,
        error: Error?
    ) {
        Task { @MainActor in
            connectedPeripheral = nil
            writeCharacteristic = nil
            notifyCharacteristic = nil
            if let error {
                connectionState = .error("Adapter disconnected: \(error.localizedDescription)")
            } else {
                connectionState = .disconnected
            }
            finishPendingCommand(with: .failure(BackendError.transport("Adapter disconnected before command completed.")))
        }
    }
}

extension CoreBluetoothOBDClient: CBPeripheralDelegate {
    nonisolated func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
        Task { @MainActor in
            if let error {
                connectionState = .error("Failed to discover services: \(error.localizedDescription)")
                return
            }

            for service in peripheral.services ?? [] {
                peripheral.discoverCharacteristics(nil, for: service)
            }
        }
    }

    nonisolated func peripheral(
        _ peripheral: CBPeripheral,
        didDiscoverCharacteristicsFor service: CBService,
        error: Error?
    ) {
        Task { @MainActor in
            if let error {
                connectionState = .error("Failed to discover characteristics for \(service.uuid.uuidString): \(error.localizedDescription)")
                return
            }
            refreshIOCharacteristics(for: peripheral)
        }
    }

    nonisolated func peripheral(
        _: CBPeripheral,
        didUpdateValueFor characteristic: CBCharacteristic,
        error: Error?
    ) {
        Task { @MainActor in
            if let error {
                finishPendingCommand(with: .failure(BackendError.transport("Adapter read failed: \(error.localizedDescription)")))
                return
            }

            guard characteristic.uuid == notifyCharacteristic?.uuid,
                  let data = characteristic.value,
                  !data.isEmpty
            else {
                return
            }

            let chunk = String(decoding: data, as: UTF8.self)
            pendingCommandBuffer.append(chunk)

            if pendingCommandBuffer.contains(">") {
                let normalized = pendingCommandBuffer
                    .replacingOccurrences(of: ">", with: "")
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                pendingCommandBuffer = normalized
                finishPendingCommand(with: .success(normalized))
            }
        }
    }
}

private extension CBPeripheral {
    var readableName: String {
        if let name, !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return name
        }
        return "Unnamed OBD Adapter"
    }
}

private extension SHA256.Digest {
    var hexString: String {
        map { String(format: "%02x", $0) }.joined()
    }
}
