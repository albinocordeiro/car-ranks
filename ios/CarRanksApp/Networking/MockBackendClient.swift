import Foundation

private enum MockScenario {
    case success
    case empty
    case error
    case loading
    case errorThenSuccess
}

private enum MockEndpoint {
    case kpiMe
    case kpiCharging
    case kpiReadiness
    case kpiTemperatureImpact
    case rankings
}

/// Deterministic mock backend behavior for fast visual iteration checkpoints.
@MainActor
final class MockBackendClient: BackendClient {
    private let decoder: JSONDecoder
    private let captureScenario: CaptureScenario
    private var kpiMeFetchCount = 0

    init(captureScenario: CaptureScenario) {
        let decoder = JSONDecoder()
        self.decoder = decoder
        self.captureScenario = captureScenario
    }

    func fetchKpisMe(vehicleUid _: UUID, timeframe _: String, temperatureBin _: String) async throws -> GenericKpiResponse {
        kpiMeFetchCount += 1
        switch scenario(for: .kpiMe) {
        case .success:
            return try decodeFixture(named: "kpis-me-response", as: GenericKpiResponse.self)
        case .empty:
            return try decodeFixture(named: "kpis-me-empty", as: GenericKpiResponse.self)
        case .error:
            throw BackendError.server(statusCode: 500, message: "Mock error state")
        case .loading:
            try await Task.sleep(nanoseconds: 20_000_000_000)
            return try decodeFixture(named: "kpis-me-response", as: GenericKpiResponse.self)
        case .errorThenSuccess:
            if kpiMeFetchCount == 1 {
                throw BackendError.server(statusCode: 500, message: "Mock retry required")
            }
            // Keep loading visible long enough so UI tests can assert the transition.
            try await Task.sleep(nanoseconds: 2_000_000_000)
            return try decodeFixture(named: "kpis-me-response", as: GenericKpiResponse.self)
        }
    }

    func fetchKpisCharging(vehicleUid _: UUID, timeframe _: String, temperatureBin _: String) async throws -> GenericKpiResponse {
        switch scenario(for: .kpiCharging) {
        case .success, .errorThenSuccess:
            return try decodeFixture(named: "kpis-charging-response", as: GenericKpiResponse.self)
        case .empty:
            return try decodeFixture(named: "kpis-charging-empty", as: GenericKpiResponse.self)
        case .error:
            throw BackendError.server(statusCode: 500, message: "Mock charging error state")
        case .loading:
            try await Task.sleep(nanoseconds: 20_000_000_000)
            return try decodeFixture(named: "kpis-charging-response", as: GenericKpiResponse.self)
        }
    }

    func fetchKpisReadiness(vehicleUid _: UUID, timeframe _: String) async throws -> ReadinessResponse {
        switch scenario(for: .kpiReadiness) {
        case .success, .errorThenSuccess:
            return try decodeFixture(named: "kpis-readiness-response", as: ReadinessResponse.self)
        case .empty:
            return try decodeFixture(named: "kpis-readiness-empty", as: ReadinessResponse.self)
        case .error:
            throw BackendError.server(statusCode: 500, message: "Mock readiness error state")
        case .loading:
            try await Task.sleep(nanoseconds: 20_000_000_000)
            return try decodeFixture(named: "kpis-readiness-response", as: ReadinessResponse.self)
        }
    }

    func fetchKpisTemperatureImpact(vehicleUid _: UUID, timeframe _: String, baseline _: String, compare _: String) async throws -> TemperatureImpactResponse {
        switch scenario(for: .kpiTemperatureImpact) {
        case .success, .errorThenSuccess:
            return try decodeFixture(named: "kpis-temperature-impact-response", as: TemperatureImpactResponse.self)
        case .empty:
            return try decodeFixture(named: "kpis-temperature-impact-empty", as: TemperatureImpactResponse.self)
        case .error:
            throw BackendError.server(statusCode: 500, message: "Mock temperature impact error state")
        case .loading:
            try await Task.sleep(nanoseconds: 20_000_000_000)
            return try decodeFixture(named: "kpis-temperature-impact-response", as: TemperatureImpactResponse.self)
        }
    }

    func fetchRankings(rankingType _: String, timeframe _: String, temperatureBin _: String, limit _: Int, offset _: Int) async throws -> RankingsResponse {
        switch scenario(for: .rankings) {
        case .success, .errorThenSuccess:
            return try decodeFixture(named: "rankings-response", as: RankingsResponse.self)
        case .empty:
            return try decodeFixture(named: "rankings-empty", as: RankingsResponse.self)
        case .error:
            throw BackendError.server(statusCode: 500, message: "Mock rankings error state")
        case .loading:
            try await Task.sleep(nanoseconds: 20_000_000_000)
            return try decodeFixture(named: "rankings-response", as: RankingsResponse.self)
        }
    }

    private func decodeFixture<T: Decodable>(named name: String, as type: T.Type) throws -> T {
        let data = try FixtureLoader.loadFixture(named: name)
        return try decoder.decode(type, from: data)
    }

    /// Each endpoint maps only to its own capture scenarios so screens do not interfere.
    private func scenario(for endpoint: MockEndpoint) -> MockScenario {
        switch endpoint {
        case .kpiMe:
            switch captureScenario {
            case .kpiMeLoading:
                return .loading
            case .kpiMeEmpty:
                return .empty
            case .kpiMeError:
                return .error
            case .kpiMeErrorThenSuccess:
                return .errorThenSuccess
            default:
                return .success
            }
        case .kpiCharging:
            switch captureScenario {
            case .kpiChargingLoading:
                return .loading
            case .kpiChargingEmpty:
                return .empty
            case .kpiChargingError:
                return .error
            default:
                return .success
            }
        case .kpiReadiness:
            switch captureScenario {
            case .kpiReadinessLoading:
                return .loading
            case .kpiReadinessEmpty:
                return .empty
            case .kpiReadinessError:
                return .error
            default:
                return .success
            }
        case .kpiTemperatureImpact:
            switch captureScenario {
            case .kpiTemperatureImpactLoading:
                return .loading
            case .kpiTemperatureImpactEmpty:
                return .empty
            case .kpiTemperatureImpactError:
                return .error
            default:
                return .success
            }
        case .rankings:
            switch captureScenario {
            case .rankingsLoading:
                return .loading
            case .rankingsEmpty:
                return .empty
            case .rankingsError:
                return .error
            default:
                return .success
            }
        }
    }
}
