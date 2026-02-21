import XCTest
@testable import CarRanksApp

final class ContractDecodingTests: XCTestCase {
    func testDecodeKpisMeResponse() throws {
        let data = try TestFixtureLoader.loadFixture(named: "kpis-me-response")
        let response = try TestFixtureLoader.decoder().decode(GenericKpiResponse.self, from: data)

        XCTAssertEqual(response.rankingType, "ev_range_efficiency")
        XCTAssertEqual(response.temperatureBin, "all")
        XCTAssertEqual(response.kpis.count, 7)
    }

    func testDecodeKpisChargingResponse() throws {
        let data = try TestFixtureLoader.loadFixture(named: "kpis-charging-response")
        let response = try TestFixtureLoader.decoder().decode(GenericKpiResponse.self, from: data)

        XCTAssertEqual(response.rankingType, "ev_charging_performance")
        XCTAssertEqual(response.kpis.count, 3)
    }

    func testDecodeKpisChargingEmptyResponse() throws {
        let data = try TestFixtureLoader.loadFixture(named: "kpis-charging-empty")
        let response = try TestFixtureLoader.decoder().decode(GenericKpiResponse.self, from: data)

        XCTAssertEqual(response.rankingType, "ev_charging_performance")
        XCTAssertTrue(response.kpis.isEmpty)
    }

    func testDecodeKpisReadinessResponse() throws {
        let data = try TestFixtureLoader.loadFixture(named: "kpis-readiness-response")
        let response = try TestFixtureLoader.decoder().decode(ReadinessResponse.self, from: data)

        XCTAssertEqual(response.timeframe, "90d")
        XCTAssertEqual(response.families.count, 4)
    }

    func testDecodeKpisReadinessEmptyResponse() throws {
        let data = try TestFixtureLoader.loadFixture(named: "kpis-readiness-empty")
        let response = try TestFixtureLoader.decoder().decode(ReadinessResponse.self, from: data)

        XCTAssertEqual(response.timeframe, "90d")
        XCTAssertTrue(response.families.isEmpty)
    }

    func testDecodeKpisTemperatureImpactResponse() throws {
        let data = try TestFixtureLoader.loadFixture(named: "kpis-temperature-impact-response")
        let response = try TestFixtureLoader.decoder().decode(TemperatureImpactResponse.self, from: data)

        XCTAssertEqual(response.baselineTemperatureBin, "mild")
        XCTAssertEqual(response.compareTemperatureBin, "cold")
        XCTAssertEqual(response.metrics.count, 3)
    }

    func testDecodeKpisTemperatureImpactEmptyResponse() throws {
        let data = try TestFixtureLoader.loadFixture(named: "kpis-temperature-impact-empty")
        let response = try TestFixtureLoader.decoder().decode(TemperatureImpactResponse.self, from: data)

        XCTAssertEqual(response.baselineTemperatureBin, "mild")
        XCTAssertEqual(response.compareTemperatureBin, "cold")
        XCTAssertTrue(response.metrics.isEmpty)
    }

    func testDecodeRankingsResponse() throws {
        let data = try TestFixtureLoader.loadFixture(named: "rankings-response")
        let response = try TestFixtureLoader.decoder().decode(RankingsResponse.self, from: data)

        XCTAssertEqual(response.rankingType, "ev_temperature_impact")
        XCTAssertEqual(response.rows.count, 1)
        XCTAssertEqual(response.page.limit, 10)
    }

    func testDecodeRankingsEmptyResponse() throws {
        let data = try TestFixtureLoader.loadFixture(named: "rankings-empty")
        let response = try TestFixtureLoader.decoder().decode(RankingsResponse.self, from: data)

        XCTAssertEqual(response.rankingType, "ev_temperature_impact")
        XCTAssertTrue(response.rows.isEmpty)
        XCTAssertEqual(response.page.limit, 10)
    }
}
