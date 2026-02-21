import Foundation
import Combine

@MainActor
final class KpiReadinessViewModel: ObservableObject {
    enum State: Equatable {
        case idle
        case loading
        case success(ReadinessResponse)
        case empty
        case error(String)
    }

    @Published private(set) var state: State = .idle

    private let backendClient: BackendClient
    private let sessionProvider: () -> SessionContext
    private let captureScenario: CaptureScenario
    private let timeframe: String
    private var hasLoadedOnce = false

    init(
        captureScenario: CaptureScenario,
        timeframe: String = "90d",
        sessionProvider: @escaping () -> SessionContext,
        backendClient: BackendClient
    ) {
        self.captureScenario = captureScenario
        self.timeframe = timeframe
        self.sessionProvider = sessionProvider
        self.backendClient = backendClient
    }

    /// First appearance path to prevent redundant reloads while navigating back and forth.
    func loadIfNeeded() {
        guard !hasLoadedOnce else { return }
        hasLoadedOnce = true
        Task {
            await load()
        }
    }

    /// Explicit refresh path used by retry and toolbar action.
    func refresh() {
        Task {
            await load()
        }
    }

    private func load() async {
        state = .loading

        // Checkpoint screenshots need the loading frame to stay stable.
        if captureScenario == .kpiReadinessLoading {
            return
        }

        let session = sessionProvider()
        do {
            let response = try await backendClient.fetchKpisReadiness(
                vehicleUid: session.vehicleUID,
                timeframe: timeframe
            )
            state = response.families.isEmpty ? .empty : .success(response)
        } catch let backendError as BackendError {
            state = .error(backendError.displayMessage)
        } catch {
            state = .error(error.localizedDescription)
        }
    }
}
