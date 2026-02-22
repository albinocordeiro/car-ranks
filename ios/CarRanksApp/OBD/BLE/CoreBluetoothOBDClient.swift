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
    private var isNotifyChannelReady = false

    private var pendingCommandContinuation: CheckedContinuation<String, Error>?
    private var pendingCommandBuffer = ""
    private var pendingCommandID: UUID?
    private var pendingCommandTimeoutTask: Task<Void, Never>?
    private let commandTimeoutNanoseconds: UInt64
    private let reconnectPolicy: OBDReconnectPolicy
    private var reconnectAttemptCount = 0
    private var reconnectTask: Task<Void, Never>?
    private var userInitiatedDisconnect = false
    private var pendingScanWhenPoweredOn = false

    override init() {
        commandTimeoutNanoseconds = 8_000_000_000
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
        let commandLabel = trimmedCommand

        guard let commandData = "\(trimmedCommand)\r".data(using: .utf8) else {
            throw BackendError.transport("Failed to encode command.")
        }

        let writeType: CBCharacteristicWriteType = writeCharacteristic.properties.contains(.write) ? .withResponse : .withoutResponse

        return try await withCheckedThrowingContinuation { continuation in
            let commandID = UUID()
            pendingCommandContinuation = continuation
            pendingCommandBuffer = ""
            pendingCommandID = commandID
            pendingCommandTimeoutTask?.cancel()
            pendingCommandTimeoutTask = Task { [weak self] in
                guard let self else { return }
                try? await Task.sleep(nanoseconds: self.commandTimeoutNanoseconds)
                await MainActor.run {
                    let bufferedResponse = self.pendingCommandBuffer
                        .trimmingCharacters(in: .whitespacesAndNewlines)
                    if !bufferedResponse.isEmpty {
                        // Some adapters return data without a terminating prompt; use what we collected.
                        self.finishPendingCommand(
                            for: commandID,
                            with: .success(bufferedResponse)
                        )
                    } else {
                        self.finishPendingCommand(
                            for: commandID,
                            with: .failure(
                                BackendError.transport(
                                    "OBD adapter timeout while waiting for response to \(commandLabel)."
                                )
                            )
                        )
                    }
                }
            }
            peripheral.writeValue(commandData, for: writeCharacteristic, type: writeType)
        }
    }

    private func updateDiscoveredDevice(
        peripheral: CBPeripheral,
        advertisedServiceUUIDs: [String],
        advertisedLocalName: String?,
        isConnectable: Bool,
        rssi: Int
    ) {
        let discoveredName = Self.resolveDiscoveredName(
            peripheralName: peripheral.name,
            advertisedLocalName: advertisedLocalName
        )
        guard OBDAdapterDiscoveryFilter.isLikelyOBDAdapter(
            name: discoveredName,
            advertisedServiceUUIDs: advertisedServiceUUIDs,
            isConnectable: isConnectable
        ) else {
            return
        }

        discoveredPeripheralByID[peripheral.identifier] = peripheral

        let device = OBDAdapterDevice(
            id: peripheral.identifier,
            name: discoveredName,
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
        guard !services.isEmpty else {
            return
        }

        // Wait until all services report their characteristic inventory so we can pick a coherent pair.
        if services.contains(where: { $0.characteristics == nil }) {
            return
        }

        let prioritizedServices = services.sorted { lhs, rhs in
            servicePriority(lhs) < servicePriority(rhs)
        }

        // First, try to keep write/notify on the same service to avoid mismatched UART channels.
        let pairedCharacteristics = prioritizedServices.compactMap(ioCharacteristics(in:)).first
        if let pairedCharacteristics {
            apply(ioCharacteristics: pairedCharacteristics)
            armNotifyChannel(for: peripheral)
            return
        }

        // Last-resort fallback for unusual adapters that split write/notify across services.
        let allCharacteristics = prioritizedServices.flatMap { $0.characteristics ?? [] }
        guard let writeCharacteristic = allCharacteristics.first(where: isWriteCharacteristic),
              let notifyCharacteristic = allCharacteristics.first(where: isNotifyCharacteristic)
        else {
            connectionState = .error("Connected adapter does not expose expected BLE UART characteristics.")
            return
        }

        apply(ioCharacteristics: (write: writeCharacteristic, notify: notifyCharacteristic))
        armNotifyChannel(for: peripheral)
    }

    private func servicePriority(_ service: CBService) -> Int {
        OBDBLEConstants.candidateServiceUUIDs.contains(service.uuid) ? 0 : 1
    }

    private func ioCharacteristics(in service: CBService) -> (write: CBCharacteristic, notify: CBCharacteristic)? {
        guard let characteristics = service.characteristics else {
            return nil
        }
        let dualPurpose = characteristics
            .filter { isWriteCharacteristic($0) && isNotifyCharacteristic($0) }
            .sorted(by: compareDualPurposeCharacteristic(_:_:))
        if let sharedCharacteristic = dualPurpose.first {
            return (write: sharedCharacteristic, notify: sharedCharacteristic)
        }

        let writeCandidates = characteristics
            .filter(isWriteCharacteristic)
            .sorted(by: compareWriteCharacteristic(_:_:))
        let notifyCandidates = characteristics
            .filter(isNotifyCharacteristic)
            .sorted(by: compareNotifyCharacteristic(_:_:))

        guard let write = writeCandidates.first,
              let notify = notifyCandidates.first
        else {
            return nil
        }
        return (write: write, notify: notify)
    }

    private func apply(ioCharacteristics: (write: CBCharacteristic, notify: CBCharacteristic)) {
        writeCharacteristic = ioCharacteristics.write
        notifyCharacteristic = ioCharacteristics.notify
        isNotifyChannelReady = false
    }

    private func armNotifyChannel(for peripheral: CBPeripheral) {
        guard let notifyCharacteristic else {
            connectionState = .error("Connected adapter does not expose expected BLE UART characteristics.")
            return
        }

        if notifyCharacteristic.isNotifying {
            isNotifyChannelReady = true
            connectionState = .connected(peripheral.readableName)
            return
        }

        connectionState = .connecting(peripheral.readableName)
        peripheral.setNotifyValue(true, for: notifyCharacteristic)
    }

    private func isWriteCharacteristic(_ characteristic: CBCharacteristic) -> Bool {
        characteristic.properties.contains(.write) || characteristic.properties.contains(.writeWithoutResponse)
    }

    private func isNotifyCharacteristic(_ characteristic: CBCharacteristic) -> Bool {
        characteristic.properties.contains(.notify) || characteristic.properties.contains(.indicate)
    }

    private func compareWriteCharacteristic(_ lhs: CBCharacteristic, _ rhs: CBCharacteristic) -> Bool {
        compareCharacteristic(lhs, rhs, preferredUUIDs: OBDBLEConstants.preferredWriteCharacteristicUUIDs)
    }

    private func compareNotifyCharacteristic(_ lhs: CBCharacteristic, _ rhs: CBCharacteristic) -> Bool {
        compareCharacteristic(lhs, rhs, preferredUUIDs: OBDBLEConstants.preferredNotifyCharacteristicUUIDs)
    }

    private func compareDualPurposeCharacteristic(_ lhs: CBCharacteristic, _ rhs: CBCharacteristic) -> Bool {
        let lhsRank = min(
            preferenceRank(of: lhs, in: OBDBLEConstants.preferredWriteCharacteristicUUIDs),
            preferenceRank(of: lhs, in: OBDBLEConstants.preferredNotifyCharacteristicUUIDs)
        )
        let rhsRank = min(
            preferenceRank(of: rhs, in: OBDBLEConstants.preferredWriteCharacteristicUUIDs),
            preferenceRank(of: rhs, in: OBDBLEConstants.preferredNotifyCharacteristicUUIDs)
        )
        if lhsRank != rhsRank {
            return lhsRank < rhsRank
        }
        return lhs.uuid.uuidString < rhs.uuid.uuidString
    }

    private func compareCharacteristic(
        _ lhs: CBCharacteristic,
        _ rhs: CBCharacteristic,
        preferredUUIDs: [CBUUID]
    ) -> Bool {
        let lhsRank = preferenceRank(of: lhs, in: preferredUUIDs)
        let rhsRank = preferenceRank(of: rhs, in: preferredUUIDs)
        if lhsRank != rhsRank {
            return lhsRank < rhsRank
        }
        return lhs.uuid.uuidString < rhs.uuid.uuidString
    }

    private func preferenceRank(of characteristic: CBCharacteristic, in preferredUUIDs: [CBUUID]) -> Int {
        if let index = preferredUUIDs.firstIndex(where: { $0 == characteristic.uuid }) {
            return index
        }
        return preferredUUIDs.count + 100
    }

    private func finishPendingCommand(
        for commandID: UUID? = nil,
        with result: Result<String, Error>
    ) {
        if let commandID, pendingCommandID != commandID {
            return
        }

        pendingCommandTimeoutTask?.cancel()
        pendingCommandTimeoutTask = nil

        guard let continuation = pendingCommandContinuation else {
            return
        }

        pendingCommandContinuation = nil
        pendingCommandID = nil
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
        let advertisedLocalName = advertisementData[CBAdvertisementDataLocalNameKey] as? String
        let isConnectable = (advertisementData[CBAdvertisementDataIsConnectable] as? NSNumber)?.boolValue ?? true
        let rssiValue = RSSI.intValue

        Task { @MainActor in
            updateDiscoveredDevice(
                peripheral: peripheral,
                advertisedServiceUUIDs: serviceUUIDs,
                advertisedLocalName: advertisedLocalName,
                isConnectable: isConnectable,
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
            isNotifyChannelReady = false
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
            isNotifyChannelReady = false
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
            isNotifyChannelReady = false
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
        _ peripheral: CBPeripheral,
        didUpdateNotificationStateFor characteristic: CBCharacteristic,
        error: Error?
    ) {
        Task { @MainActor in
            guard characteristic.uuid == notifyCharacteristic?.uuid else {
                return
            }

            if let error {
                isNotifyChannelReady = false
                connectionState = .error("Failed to subscribe to adapter notifications: \(error.localizedDescription)")
                return
            }

            if characteristic.isNotifying {
                isNotifyChannelReady = true
                connectionState = .connected(peripheral.readableName)
            } else {
                isNotifyChannelReady = false
                connectionState = .error("Adapter notifications were disabled unexpectedly.")
            }
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

private extension CoreBluetoothOBDClient {
    static func resolveDiscoveredName(peripheralName: String?, advertisedLocalName: String?) -> String {
        if let advertisedLocalName, !advertisedLocalName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return advertisedLocalName
        }
        if let peripheralName, !peripheralName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return peripheralName
        }
        return "Unnamed OBD Adapter"
    }
}

private extension SHA256.Digest {
    var hexString: String {
        map { String(format: "%02x", $0) }.joined()
    }
}
