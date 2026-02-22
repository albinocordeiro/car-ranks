import Foundation
import Network

/// Owns durable telemetry queueing plus automatic retry when app/network conditions improve.
@MainActor
final class TelemetryUploadQueueCoordinator: ObservableObject {
    @Published private(set) var pendingBatchCount = 0
    @Published private(set) var isUploading = false
    @Published private(set) var lastUploadMessage: String?
    @Published private(set) var lastQueuedBatchSummary: String?
    @Published private(set) var lastSuccessfulUploadSummary: String?
    @Published private(set) var lastSuccessfulUploadResponse: TelemetryBatchUploadResponse?
    @Published private(set) var lastSuccessfulUploadAt: Date?
    @Published private(set) var nextRetryInSeconds: Int?

    private let uploader: TelemetryBatchUploader
    private let store: TelemetryUploadQueueStore
    private let retryPolicy: TelemetryUploadRetryPolicy
    private let now: () -> Date
    private let autoUploadEnabled: Bool

    private var queue: [TelemetryPendingBatch]
    private var flushTask: Task<Void, Never>?
    private var retryWakeTask: Task<Void, Never>?
    private var retryCountdownTask: Task<Void, Never>?

    private var pathMonitor: NWPathMonitor?
    private let pathMonitorQueue = DispatchQueue(label: "com.albinocordeiro.carranks.telemetry.queue.monitor")

    init(
        uploader: TelemetryBatchUploader,
        store: TelemetryUploadQueueStore = FileTelemetryUploadQueueStore(),
        retryPolicy: TelemetryUploadRetryPolicy = .standard,
        now: @escaping () -> Date = Date.init,
        autoUploadEnabled: Bool = true,
        enableNetworkMonitoring: Bool = true
    ) {
        self.uploader = uploader
        self.store = store
        self.retryPolicy = retryPolicy
        self.now = now
        self.autoUploadEnabled = autoUploadEnabled

        do {
            queue = try store.load()
        } catch {
            queue = []
            lastUploadMessage = "Failed to read queued telemetry batches: \(error.localizedDescription)"
        }
        pendingBatchCount = queue.count

        if enableNetworkMonitoring {
            startNetworkMonitor()
        }

        if autoUploadEnabled {
            scheduleFlush(force: false)
        } else {
            scheduleRetryWakeIfNeeded()
        }
    }

    deinit {
        flushTask?.cancel()
        retryWakeTask?.cancel()
        retryCountdownTask?.cancel()
        pathMonitor?.cancel()
    }

    func enqueue(batch: TelemetryBatchRequest, captureWindowEndedAt: Date) {
        queue.append(
            TelemetryPendingBatch(
                request: batch,
                captureWindowEndedAt: captureWindowEndedAt,
                enqueuedAt: now()
            )
        )
        persistQueue()
        pendingBatchCount = queue.count
        lastQueuedBatchSummary = "Batch \(Self.shortBatchID(batch.batchID)): \(Self.payloadSummary(for: batch))"
        lastUploadMessage = "Queued telemetry batch for upload."
        refreshRetryCountdownState()
        scheduleFlush(force: false)
    }

    /// Manual trigger used by the UI for explicit retry intent.
    func triggerUpload() {
        scheduleFlush(force: true)
    }

    /// Called when app comes to foreground so queued data drains quickly.
    func appDidBecomeActive() {
        scheduleFlush(force: false)
    }

    private func scheduleFlush(force: Bool) {
        guard force || autoUploadEnabled else {
            return
        }
        guard flushTask == nil else {
            return
        }

        retryWakeTask?.cancel()
        retryWakeTask = nil
        flushTask = Task { [weak self] in
            await self?.flushQueue()
        }
    }

    private func flushQueue() async {
        defer {
            flushTask = nil
            scheduleRetryWakeIfNeeded()
        }

        while !Task.isCancelled {
            guard !queue.isEmpty else {
                isUploading = false
                return
            }

            let head = queue[0]
            if let nextRetryAt = head.nextRetryAt, nextRetryAt > now() {
                isUploading = false
                return
            }

            isUploading = true
            do {
                let uploadResponse = try await uploader.upload(batch: head.request)
                queue.removeFirst()
                persistQueue()
                pendingBatchCount = queue.count
                let uploadedAt = now()
                lastSuccessfulUploadResponse = uploadResponse
                lastSuccessfulUploadAt = uploadedAt
                lastSuccessfulUploadSummary = "Batch \(Self.shortBatchID(head.request.batchID)) uploaded at \(TelemetryTimestampFormatter.string(from: uploadedAt))"
                lastUploadMessage = "Uploaded queued telemetry batch."
                refreshRetryCountdownState()
            } catch let backendError as BackendError {
                handleUploadFailure(error: backendError, head: head)
                return
            } catch {
                handleUploadFailure(error: .transport(error.localizedDescription), head: head)
                return
            }
        }
    }

    private func handleUploadFailure(error: BackendError, head: TelemetryPendingBatch) {
        isUploading = false
        guard !queue.isEmpty else {
            return
        }

        if retryPolicy.shouldRetry(error: error, retryCount: head.retryCount) {
            var updated = head
            let updatedRetryCount = head.retryCount + 1
            updated.retryCount = updatedRetryCount
            let delaySeconds = retryPolicy.delaySeconds(forRetryCount: updatedRetryCount)
            updated.nextRetryAt = now().addingTimeInterval(delaySeconds)
            updated.lastErrorMessage = error.displayMessage
            queue[0] = updated
            persistQueue()
            lastUploadMessage = "Telemetry upload failed. Retry \(updatedRetryCount)/\(retryPolicy.maxAttempts) scheduled in \(Int(delaySeconds))s."
            refreshRetryCountdownState()
            return
        }

        queue.removeFirst()
        persistQueue()
        pendingBatchCount = queue.count
        lastUploadMessage = "Dropped queued telemetry batch: \(error.displayMessage)"
        refreshRetryCountdownState()
    }

    private func persistQueue() {
        do {
            try store.save(queue)
        } catch {
            lastUploadMessage = "Failed to persist upload queue: \(error.localizedDescription)"
        }
    }

    private func scheduleRetryWakeIfNeeded() {
        retryWakeTask?.cancel()
        retryWakeTask = nil

        guard autoUploadEnabled, let nextRetryAt = queue.first?.nextRetryAt else {
            refreshRetryCountdownState()
            return
        }

        refreshRetryCountdownState()
        let delaySeconds = max(0, nextRetryAt.timeIntervalSince(now()))
        let delayNanoseconds = UInt64(delaySeconds * 1_000_000_000)
        retryWakeTask = Task { [weak self] in
            if delayNanoseconds > 0 {
                try? await Task.sleep(nanoseconds: delayNanoseconds)
            }
            await MainActor.run {
                self?.scheduleFlush(force: false)
            }
        }
    }

    private func refreshRetryCountdownState() {
        guard let nextRetryAt = queue.first?.nextRetryAt else {
            nextRetryInSeconds = nil
            retryCountdownTask?.cancel()
            retryCountdownTask = nil
            return
        }

        let remaining = max(0, Int(ceil(nextRetryAt.timeIntervalSince(now()))))
        nextRetryInSeconds = remaining
        if remaining == 0 {
            retryCountdownTask?.cancel()
            retryCountdownTask = nil
            return
        }

        if retryCountdownTask != nil {
            return
        }

        retryCountdownTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 1_000_000_000)
                await MainActor.run {
                    self?.refreshRetryCountdownState()
                }
            }
        }
    }

    private func startNetworkMonitor() {
        let monitor = NWPathMonitor()
        monitor.pathUpdateHandler = { [weak self] path in
            guard path.status == .satisfied else {
                return
            }
            Task { @MainActor in
                self?.scheduleFlush(force: false)
            }
        }
        monitor.start(queue: pathMonitorQueue)
        pathMonitor = monitor
    }

    private static func payloadSummary(for batch: TelemetryBatchRequest) -> String {
        [
            countText(batch.records.count, singular: "signal"),
            countText(batch.diagnostics.count, singular: "diagnostic"),
            countText(batch.sessionEvents.count, singular: "session event"),
        ]
        .joined(separator: ", ")
    }

    private static func shortBatchID(_ batchID: UUID) -> String {
        String(batchID.uuidString.prefix(8))
    }

    private static func countText(_ count: Int, singular: String) -> String {
        if count == 1 {
            return "1 \(singular)"
        }
        return "\(count) \(singular)s"
    }
}
