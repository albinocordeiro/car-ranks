import Foundation
import Combine

@MainActor
final class KpiMeViewModel: ObservableObject {
    enum State: Equatable {
        case idle
        case loading
        case success(GenericKpiResponse)
        case empty
        case error(String)
    }

    @Published private(set) var state: State = .idle

    private let backendClient: BackendClient
    private let sessionProvider: () -> SessionContext
    private let captureScenario: CaptureScenario
    private let timeframe: String
    private let temperatureBin: String
    private var hasLoadedOnce = false

    init(
        captureScenario: CaptureScenario,
        timeframe: String = "90d",
        temperatureBin: String = "all",
        sessionProvider: @escaping () -> SessionContext,
        backendClient: BackendClient
    ) {
        self.captureScenario = captureScenario
        self.timeframe = timeframe
        self.temperatureBin = temperatureBin
        self.sessionProvider = sessionProvider
        self.backendClient = backendClient
    }

    /// Used by screen lifecycle to prevent duplicate network calls on repeated appearances.
    func loadIfNeeded() {
        guard !hasLoadedOnce else { return }
        hasLoadedOnce = true
        Task {
            await load()
        }
    }

    /// Explicit refresh path used after session edits or retry action.
    func refresh() {
        Task {
            await load()
        }
    }

    private func load() async {
        state = .loading

        // Loading screenshot checkpoints must remain stable long enough for capture.
        if captureScenario == .kpiMeLoading {
            return
        }

        let session = sessionProvider()
        do {
            let response = try await backendClient.fetchKpisMe(
                vehicleUid: session.vehicleUID,
                timeframe: timeframe,
                temperatureBin: temperatureBin
            )
            state = response.kpis.isEmpty ? .empty : .success(response)
        } catch let backendError as BackendError {
            state = .error(backendError.displayMessage)
        } catch {
            state = .error(error.localizedDescription)
        }
    }
}
