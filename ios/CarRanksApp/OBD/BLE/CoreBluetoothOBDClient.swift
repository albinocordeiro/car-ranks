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
    private let reconnectPolicy: OBDReconnectPolicy
    private var reconnectAttemptCount = 0
    private var reconnectTask: Task<Void, Never>?
    private var userInitiatedDisconnect = false
    private var pendingScanWhenPoweredOn = false

    override init() {
        commandTimeoutNanoseconds = 5_000_000_000
        reconnectPolicy = .standard
        super.init()
        centralManager = CBCentralManager(delegate: self, queue: nil)
    }

    init(
        commandTimeoutNanoseconds: UInt64,
        reconnectPolicy: OBDReconnectPolicy = .standard
    ) {
        self.commandTimeoutNanoseconds = commandTimeoutNanoseconds
        self.reconnectPolicy = reconnectPolicy
        super.init()
        centralManager = CBCentralManager(delegate: self, queue: nil)
    }

    func startScanning() {
        guard centralManager.state == .poweredOn else {
            pendingScanWhenPoweredOn = true
            connectionState = .error(
                OBDBluetoothStateMessage.forState(centralManager.state) ?? "Bluetooth is unavailable."
            )
            return
        }
        pendingScanWhenPoweredOn = false
        performScan()
    }

    private func performScan() {

        // A new scan indicates a fresh user intent, so any pending reconnect loop is canceled.
        cancelReconnectTask()
        reconnectAttemptCount = 0
        userInitiatedDisconnect = false

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
        pendingScanWhenPoweredOn = false
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

        cancelReconnectTask()
        reconnectAttemptCount = 0
        userInitiatedDisconnect = false
        stopScanning()
        connectionState = .connecting(peripheral.readableName)
        centralManager.connect(peripheral, options: nil)
    }

    func disconnect() {
        userInitiatedDisconnect = true
        cancelReconnectTask()
        reconnectAttemptCount = 0

        guard let connectedPeripheral else {
            connectionState = .disconnected
            return
        }

        finishPendingCommand(with: .failure(BackendError.transport("Adapter disconnected by user.")))
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

    private func cancelReconnectTask() {
        reconnectTask?.cancel()
        reconnectTask = nil
    }

    private func scheduleReconnect(for peripheral: CBPeripheral) -> Bool {
        guard centralManager.state == .poweredOn, !userInitiatedDisconnect else {
            return false
        }

        let attempt = reconnectAttemptCount + 1
        guard reconnectPolicy.shouldRetry(attempt: attempt) else {
            return false
        }

        reconnectAttemptCount = attempt
        let delaySeconds = reconnectPolicy.delaySeconds(forAttempt: attempt)
        let delayNanoseconds = UInt64(max(0, delaySeconds) * 1_000_000_000)
        connectionState = .reconnecting(
            name: peripheral.readableName,
            attempt: attempt,
            maxAttempts: reconnectPolicy.maxAttempts
        )

        cancelReconnectTask()
        reconnectTask = Task { [weak self] in
            if delayNanoseconds > 0 {
                try? await Task.sleep(nanoseconds: delayNanoseconds)
            }
            await MainActor.run {
                guard let self, !self.userInitiatedDisconnect else { return }
                guard self.centralManager.state == .poweredOn else {
                    self.connectionState = .error("Bluetooth is not powered on.")
                    return
                }
                self.connectionState = .connecting(peripheral.readableName)
                self.centralManager.connect(peripheral, options: nil)
            }
        }
        return true
    }
}

extension CoreBluetoothOBDClient: CBCentralManagerDelegate {
    nonisolated func centralManagerDidUpdateState(_ central: CBCentralManager) {
        Task { @MainActor in
            if let message = OBDBluetoothStateMessage.forState(central.state) {
                let hadActiveBluetoothFlow = pendingScanWhenPoweredOn || connectedPeripheral != nil || {
                    switch connectionState {
                    case .scanning, .connecting, .reconnecting, .connected, .error:
                        return true
                    case .disconnected:
                        return false
                    }
                }()

                cancelReconnectTask()
                centralManager.stopScan()
                if hadActiveBluetoothFlow {
                    connectionState = .error(message)
                } else {
                    connectionState = .disconnected
                }
                return
            }

            if pendingScanWhenPoweredOn {
                pendingScanWhenPoweredOn = false
                performScan()
                return
            }

            if case .error = connectionState {
                connectionState = .disconnected
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
            cancelReconnectTask()
            reconnectAttemptCount = 0
            userInitiatedDisconnect = false
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
            if scheduleReconnect(for: peripheral) {
                return
            }
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
            finishPendingCommand(with: .failure(BackendError.transport("Adapter disconnected before command completed.")))

            if userInitiatedDisconnect {
                connectionState = .disconnected
                return
            }

            if scheduleReconnect(for: peripheral) {
                return
            }

            if let error {
                connectionState = .error("Adapter disconnected: \(error.localizedDescription)")
            } else {
                connectionState = .error("Adapter disconnected unexpectedly.")
            }
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
