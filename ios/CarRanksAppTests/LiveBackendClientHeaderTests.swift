import Foundation
import XCTest
@testable import CarRanksApp

@MainActor
private final class RecordingNetworkSession: NetworkSession {
    private(set) var lastRequest: URLRequest?
    private let dataToReturn: Data
    private let statusCode: Int

    init(dataToReturn: Data, statusCode: Int = 200) {
        self.dataToReturn = dataToReturn
        self.statusCode = statusCode
    }

    func data(for request: URLRequest) async throws -> (Data, URLResponse) {
        lastRequest = request
        let response = HTTPURLResponse(
            url: request.url ?? URL(string: "http://localhost")!,
            statusCode: statusCode,
            httpVersion: nil,
            headerFields: nil
        )!
        return (dataToReturn, response)
    }
}

@MainActor
final class LiveBackendClientHeaderTests: XCTestCase {
    func testFetchKpisMeInjectsXUserIDHeader() async throws {
        let userID = UUID(uuidString: "11111111-1111-1111-1111-111111111111")!
        let vehicleUID = UUID(uuidString: "22222222-2222-2222-2222-222222222222")!
        let responseFixture = try TestFixtureLoader.loadFixture(named: "kpis-me-response")
        let session = RecordingNetworkSession(dataToReturn: responseFixture)

        let client = LiveBackendClient(
            baseURL: URL(string: "http://localhost:8080")!,
            sessionProvider: { SessionContext(userID: userID, vehicleUID: vehicleUID) },
            networkSession: session
        )

        _ = try await client.fetchKpisMe(vehicleUid: vehicleUID, timeframe: "90d", temperatureBin: "all")

        let request = session.lastRequest
        XCTAssertEqual(request?.value(forHTTPHeaderField: "x-user-id"), userID.uuidString)
        XCTAssertTrue(request?.url?.path.contains("/v1/kpis/me") ?? false)
    }
}
