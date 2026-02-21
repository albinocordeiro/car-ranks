import Foundation

/// Decorator for live backend calls used only during screenshot checkpoints.
/// It forces deterministic empty/error states for capture scenarios while still
/// delegating success paths to the real backend.
@MainActor
final class LiveCaptureOverrideBackendClient: BackendClient {
    private let liveClient: BackendClient
    private let captureScenario: CaptureScenario
    private let decoder: JSONDecoder

    init(liveClient: BackendClient, captureScenario: CaptureScenario) {
        self.liveClient = liveClient
        self.captureScenario = captureScenario
        let decoder = JSONDecoder()
        self.decoder = decoder
    }

    func fetchKpisMe(vehicleUid: UUID, timeframe: String, temperatureBin: String) async throws -> GenericKpiResponse {
        switch captureScenario {
        case .kpiMeEmpty:
            return try decodeFixture(named: "kpis-me-empty", as: GenericKpiResponse.self)
        case .kpiMeError:
            throw forcedError(message: "Forced live capture error for KPI Me")
        default:
            return try await liveClient.fetchKpisMe(vehicleUid: vehicleUid, timeframe: timeframe, temperatureBin: temperatureBin)
        }
    }

    func fetchKpisCharging(vehicleUid: UUID, timeframe: String, temperatureBin: String) async throws -> GenericKpiResponse {
        switch captureScenario {
        case .kpiChargingEmpty:
            return try decodeFixture(named: "kpis-charging-empty", as: GenericKpiResponse.self)
        case .kpiChargingError:
            throw forcedError(message: "Forced live capture error for KPI Charging")
        default:
            return try await liveClient.fetchKpisCharging(vehicleUid: vehicleUid, timeframe: timeframe, temperatureBin: temperatureBin)
        }
    }

    func fetchKpisReadiness(vehicleUid: UUID, timeframe: String) async throws -> ReadinessResponse {
        switch captureScenario {
        case .kpiReadinessEmpty:
            return try decodeFixture(named: "kpis-readiness-empty", as: ReadinessResponse.self)
        case .kpiReadinessError:
            throw forcedError(message: "Forced live capture error for KPI Readiness")
        default:
            return try await liveClient.fetchKpisReadiness(vehicleUid: vehicleUid, timeframe: timeframe)
        }
    }

    func fetchKpisTemperatureImpact(vehicleUid: UUID, timeframe: String, baseline: String, compare: String) async throws -> TemperatureImpactResponse {
        switch captureScenario {
        case .kpiTemperatureImpactEmpty:
            return try decodeFixture(named: "kpis-temperature-impact-empty", as: TemperatureImpactResponse.self)
        case .kpiTemperatureImpactError:
            throw forcedError(message: "Forced live capture error for KPI Temperature Impact")
        default:
            return try await liveClient.fetchKpisTemperatureImpact(vehicleUid: vehicleUid, timeframe: timeframe, baseline: baseline, compare: compare)
        }
    }

    func fetchRankings(rankingType: String, timeframe: String, temperatureBin: String, limit: Int, offset: Int) async throws -> RankingsResponse {
        switch captureScenario {
        case .rankingsEmpty:
            return try decodeFixture(named: "rankings-empty", as: RankingsResponse.self)
        case .rankingsError:
            throw forcedError(message: "Forced live capture error for Rankings")
        default:
            return try await liveClient.fetchRankings(
                rankingType: rankingType,
                timeframe: timeframe,
                temperatureBin: temperatureBin,
                limit: limit,
                offset: offset
            )
        }
    }

    private func forcedError(message: String) -> BackendError {
        .server(statusCode: 500, message: message)
    }

    private func decodeFixture<T: Decodable>(named name: String, as type: T.Type) throws -> T {
        let data = try FixtureLoader.loadFixture(named: name)
        return try decoder.decode(type, from: data)
    }
}
