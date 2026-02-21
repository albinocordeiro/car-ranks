import Foundation
import Combine

@MainActor
final class KpiChargingViewModel: ObservableObject {
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

    func loadIfNeeded() {
        guard !hasLoadedOnce else { return }
        hasLoadedOnce = true
        Task {
            await load()
        }
    }

    func refresh() {
        Task {
            await load()
        }
    }

    private func load() async {
        state = .loading

        // Capture runs need a stable loading frame before any response transition.
        if captureScenario == .kpiChargingLoading {
            return
        }

        let session = sessionProvider()
        do {
            let response = try await backendClient.fetchKpisCharging(
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
