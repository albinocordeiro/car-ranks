import Foundation

@MainActor
protocol NetworkSession {
    func data(for request: URLRequest) async throws -> (Data, URLResponse)
}

extension URLSession: NetworkSession {}

/// Real backend implementation. The request builder centralizes x-user-id injection.
@MainActor
final class LiveBackendClient: BackendClient {
    private let baseURL: URL
    private let sessionProvider: () -> SessionContext
    private let networkSession: NetworkSession
    private let decoder: JSONDecoder

    init(
        baseURL: URL,
        sessionProvider: @escaping () -> SessionContext,
        networkSession: NetworkSession = URLSession.shared
    ) {
        self.baseURL = baseURL
        self.sessionProvider = sessionProvider
        self.networkSession = networkSession
        let decoder = JSONDecoder()
        self.decoder = decoder
    }

    func fetchKpisMe(vehicleUid: UUID, timeframe: String, temperatureBin: String) async throws -> GenericKpiResponse {
        try await get(
            path: "/v1/kpis/me",
            query: [
                URLQueryItem(name: "vehicle_uid", value: vehicleUid.uuidString),
                URLQueryItem(name: "timeframe", value: timeframe),
                URLQueryItem(name: "temperature_bin", value: temperatureBin),
            ]
        )
    }

    func fetchKpisCharging(vehicleUid: UUID, timeframe: String, temperatureBin: String) async throws -> GenericKpiResponse {
        try await get(
            path: "/v1/kpis/charging",
            query: [
                URLQueryItem(name: "vehicle_uid", value: vehicleUid.uuidString),
                URLQueryItem(name: "timeframe", value: timeframe),
                URLQueryItem(name: "temperature_bin", value: temperatureBin),
            ]
        )
    }

    func fetchKpisReadiness(vehicleUid: UUID, timeframe: String) async throws -> ReadinessResponse {
        try await get(
            path: "/v1/kpis/readiness",
            query: [
                URLQueryItem(name: "vehicle_uid", value: vehicleUid.uuidString),
                URLQueryItem(name: "timeframe", value: timeframe),
            ]
        )
    }

    func fetchKpisTemperatureImpact(vehicleUid: UUID, timeframe: String, baseline: String, compare: String) async throws -> TemperatureImpactResponse {
        try await get(
            path: "/v1/kpis/temperature-impact",
            query: [
                URLQueryItem(name: "vehicle_uid", value: vehicleUid.uuidString),
                URLQueryItem(name: "timeframe", value: timeframe),
                URLQueryItem(name: "baseline_temperature_bin", value: baseline),
                URLQueryItem(name: "compare_temperature_bin", value: compare),
            ]
        )
    }

    func fetchRankings(rankingType: String, timeframe: String, temperatureBin: String, limit: Int, offset: Int) async throws -> RankingsResponse {
        try await get(
            path: "/v1/rankings",
            query: [
                URLQueryItem(name: "ranking_type", value: rankingType),
                URLQueryItem(name: "timeframe", value: timeframe),
                URLQueryItem(name: "temperature_bin", value: temperatureBin),
                URLQueryItem(name: "limit", value: String(limit)),
                URLQueryItem(name: "offset", value: String(offset)),
            ]
        )
    }

    private func get<T: Decodable>(path: String, query: [URLQueryItem]) async throws -> T {
        var components = URLComponents(url: baseURL, resolvingAgainstBaseURL: false)
        components?.path = path
        components?.queryItems = query

        guard let finalURL = components?.url else {
            throw BackendError.invalidURL
        }

        var request = URLRequest(url: finalURL)
        request.httpMethod = "GET"
        request.timeoutInterval = 30
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.setValue(sessionProvider().userID.uuidString, forHTTPHeaderField: "x-user-id")

        do {
            let (data, response) = try await networkSession.data(for: request)
            guard let httpResponse = response as? HTTPURLResponse else {
                throw BackendError.invalidResponse
            }

            guard (200...299).contains(httpResponse.statusCode) else {
                let payload = try? decoder.decode(BackendErrorPayload.self, from: data)
                let message = payload?.message ?? "Request failed with status \(httpResponse.statusCode)."
                throw BackendError.server(statusCode: httpResponse.statusCode, message: message)
            }

            do {
                return try decoder.decode(T.self, from: data)
            } catch {
                throw BackendError.decode(error.localizedDescription)
            }
        } catch let backendError as BackendError {
            throw backendError
        } catch {
            throw BackendError.transport(error.localizedDescription)
        }
    }
}
