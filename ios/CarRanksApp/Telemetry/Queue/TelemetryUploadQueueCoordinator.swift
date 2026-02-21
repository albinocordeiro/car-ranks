import Foundation
import Network

/// Owns durable telemetry queueing plus automatic retry when app/network conditions improve.
@MainActor
final class TelemetryUploadQueueCoordinator: ObservableObject {
    @Published private(set) var pendingBatchCount = 0
    @Published private(set) var isUploading = false
    @Published private(set) var lastUploadMessage: String?

    private let uploader: TelemetryBatchUploader
    private let store: TelemetryUploadQueueStore
    private let retryPolicy: TelemetryUploadRetryPolicy
    private let now: () -> Date
    private let autoUploadEnabled: Bool

    private var queue: [TelemetryPendingBatch]
    private var flushTask: Task<Void, Never>?
    private var retryWakeTask: Task<Void, Never>?

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
        lastUploadMessage = "Queued telemetry batch for upload."
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
                _ = try await uploader.upload(batch: head.request)
                queue.removeFirst()
                persistQueue()
                pendingBatchCount = queue.count
                lastUploadMessage = "Uploaded queued telemetry batch."
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
            return
        }

        queue.removeFirst()
        persistQueue()
        pendingBatchCount = queue.count
        lastUploadMessage = "Dropped queued telemetry batch: \(error.displayMessage)"
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
            return
        }

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
}
