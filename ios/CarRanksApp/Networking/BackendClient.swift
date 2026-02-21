import Foundation

@MainActor
protocol BackendClient {
    func fetchKpisMe(vehicleUid: UUID, timeframe: String, temperatureBin: String) async throws -> GenericKpiResponse
    func fetchKpisCharging(vehicleUid: UUID, timeframe: String, temperatureBin: String) async throws -> GenericKpiResponse
    func fetchKpisReadiness(vehicleUid: UUID, timeframe: String) async throws -> ReadinessResponse
    func fetchKpisTemperatureImpact(vehicleUid: UUID, timeframe: String, baseline: String, compare: String) async throws -> TemperatureImpactResponse
    func fetchRankings(rankingType: String, timeframe: String, temperatureBin: String, limit: Int, offset: Int) async throws -> RankingsResponse
}
