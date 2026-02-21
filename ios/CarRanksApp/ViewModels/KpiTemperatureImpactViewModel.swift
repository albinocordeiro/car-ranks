import Foundation
import Combine

@MainActor
final class KpiTemperatureImpactViewModel: ObservableObject {
    enum State: Equatable {
        case idle
        case loading
        case success(TemperatureImpactResponse)
        case empty
        case error(String)
    }

    @Published private(set) var state: State = .idle

    private let backendClient: BackendClient
    private let sessionProvider: () -> SessionContext
    private let captureScenario: CaptureScenario
    private let timeframe: String
    private let baseline: String
    private let compare: String
    private var hasLoadedOnce = false

    init(
        captureScenario: CaptureScenario,
        timeframe: String = "90d",
        baseline: String = "mild",
        compare: String = "cold",
        sessionProvider: @escaping () -> SessionContext,
        backendClient: BackendClient
    ) {
        self.captureScenario = captureScenario
        self.timeframe = timeframe
        self.baseline = baseline
        self.compare = compare
        self.sessionProvider = sessionProvider
        self.backendClient = backendClient
    }

    /// First appearance path to keep navigation transitions efficient.
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
        if captureScenario == .kpiTemperatureImpactLoading {
            return
        }

        let session = sessionProvider()
        do {
            let response = try await backendClient.fetchKpisTemperatureImpact(
                vehicleUid: session.vehicleUID,
                timeframe: timeframe,
                baseline: baseline,
                compare: compare
            )
            state = response.metrics.isEmpty ? .empty : .success(response)
        } catch let backendError as BackendError {
            state = .error(backendError.displayMessage)
        } catch {
            state = .error(error.localizedDescription)
        }
    }
}
