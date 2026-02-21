import Foundation
import Combine

@MainActor
final class RankingsViewModel: ObservableObject {
    enum State: Equatable {
        case idle
        case loading
        case success(RankingsResponse)
        case empty
        case error(String)
    }

    @Published private(set) var state: State = .idle

    private let backendClient: BackendClient
    private let captureScenario: CaptureScenario
    private let rankingType: String
    private let timeframe: String
    private let temperatureBin: String
    private let limit: Int
    private let offset: Int
    private var hasLoadedOnce = false

    init(
        captureScenario: CaptureScenario,
        rankingType: String = "ev_temperature_impact",
        timeframe: String = "90d",
        temperatureBin: String = "all",
        limit: Int = 10,
        offset: Int = 0,
        backendClient: BackendClient
    ) {
        self.captureScenario = captureScenario
        self.rankingType = rankingType
        self.timeframe = timeframe
        self.temperatureBin = temperatureBin
        self.limit = limit
        self.offset = offset
        self.backendClient = backendClient
    }

    /// First appearance path to keep navigation transitions fast and predictable.
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

        // Checkpoint screenshots need a stable loading frame.
        if captureScenario == .rankingsLoading {
            return
        }

        do {
            let response = try await backendClient.fetchRankings(
                rankingType: rankingType,
                timeframe: timeframe,
                temperatureBin: temperatureBin,
                limit: limit,
                offset: offset
            )
            state = response.rows.isEmpty ? .empty : .success(response)
        } catch let backendError as BackendError {
            state = .error(backendError.displayMessage)
        } catch {
            state = .error(error.localizedDescription)
        }
    }
}
