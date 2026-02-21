import XCTest
@testable import CarRanksApp

@MainActor
final class LiveCaptureOverrideBackendClientTests: XCTestCase {
    func testKpiMeEmptyScenarioUsesFixtureInsteadOfLiveCall() async throws {
        let stub = try StubLiveBackendClient()
        let client = LiveCaptureOverrideBackendClient(liveClient: stub, captureScenario: .kpiMeEmpty)

        let response = try await client.fetchKpisMe(
            vehicleUid: UUID(uuidString: "e11889bf-504c-4238-9583-bc8840f20e19")!,
            timeframe: "90d",
            temperatureBin: "all"
        )

        XCTAssertTrue(response.kpis.isEmpty)
        XCTAssertEqual(stub.kpiMeCalls, 0)
    }

    func testKpiMeErrorScenarioThrowsForcedError() async {
        do {
            let stub = try StubLiveBackendClient()
            let client = LiveCaptureOverrideBackendClient(liveClient: stub, captureScenario: .kpiMeError)
            _ = try await client.fetchKpisMe(
                vehicleUid: UUID(uuidString: "e11889bf-504c-4238-9583-bc8840f20e19")!,
                timeframe: "90d",
                temperatureBin: "all"
            )
            XCTFail("Expected forced error")
        } catch let error as BackendError {
            if case .server(let statusCode, let message) = error {
                XCTAssertEqual(statusCode, 500)
                XCTAssertTrue(message.contains("Forced live capture error"))
            } else {
                XCTFail("Expected server error, got \(error)")
            }
        } catch {
            XCTFail("Unexpected error type: \(error)")
        }
    }

    func testKpiMeSuccessScenarioDelegatesToLiveClient() async throws {
        let stub = try StubLiveBackendClient()
        let client = LiveCaptureOverrideBackendClient(liveClient: stub, captureScenario: .kpiMeSuccess)

        let response = try await client.fetchKpisMe(
            vehicleUid: UUID(uuidString: "e11889bf-504c-4238-9583-bc8840f20e19")!,
            timeframe: "90d",
            temperatureBin: "all"
        )

        XCTAssertFalse(response.kpis.isEmpty)
        XCTAssertEqual(stub.kpiMeCalls, 1)
    }

    func testRankingsEmptyScenarioUsesFixtureInsteadOfLiveCall() async throws {
        let stub = try StubLiveBackendClient()
        let client = LiveCaptureOverrideBackendClient(liveClient: stub, captureScenario: .rankingsEmpty)

        let response = try await client.fetchRankings(
            rankingType: "ev_temperature_impact",
            timeframe: "90d",
            temperatureBin: "all",
            limit: 10,
            offset: 0
        )

        XCTAssertTrue(response.rows.isEmpty)
        XCTAssertEqual(stub.rankingsCalls, 0)
    }
}

@MainActor
private final class StubLiveBackendClient: BackendClient {
    private let meResponse: GenericKpiResponse
    private let chargingResponse: GenericKpiResponse
    private let readinessResponse: ReadinessResponse
    private let temperatureImpactResponse: TemperatureImpactResponse
    private let rankingsResponse: RankingsResponse

    private(set) var kpiMeCalls = 0
    private(set) var rankingsCalls = 0

    init() throws {
        meResponse = try Self.decodeFixture(named: "kpis-me-response", as: GenericKpiResponse.self)
        chargingResponse = try Self.decodeFixture(named: "kpis-charging-response", as: GenericKpiResponse.self)
        readinessResponse = try Self.decodeFixture(named: "kpis-readiness-response", as: ReadinessResponse.self)
        temperatureImpactResponse = try Self.decodeFixture(named: "kpis-temperature-impact-response", as: TemperatureImpactResponse.self)
        rankingsResponse = try Self.decodeFixture(named: "rankings-response", as: RankingsResponse.self)
    }

    func fetchKpisMe(vehicleUid _: UUID, timeframe _: String, temperatureBin _: String) async throws -> GenericKpiResponse {
        kpiMeCalls += 1
        return meResponse
    }

    func fetchKpisCharging(vehicleUid _: UUID, timeframe _: String, temperatureBin _: String) async throws -> GenericKpiResponse {
        chargingResponse
    }

    func fetchKpisReadiness(vehicleUid _: UUID, timeframe _: String) async throws -> ReadinessResponse {
        readinessResponse
    }

    func fetchKpisTemperatureImpact(vehicleUid _: UUID, timeframe _: String, baseline _: String, compare _: String) async throws -> TemperatureImpactResponse {
        temperatureImpactResponse
    }

    func fetchRankings(rankingType _: String, timeframe _: String, temperatureBin _: String, limit _: Int, offset _: Int) async throws -> RankingsResponse {
        rankingsCalls += 1
        return rankingsResponse
    }

    private static func decodeFixture<T: Decodable>(named name: String, as type: T.Type) throws -> T {
        let data = try TestFixtureLoader.loadFixture(named: name)
        return try TestFixtureLoader.decoder().decode(type, from: data)
    }
}
