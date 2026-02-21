import Foundation

/// Dedicated client for telemetry ingestion so KPI APIs remain isolated and reviewer-friendly.
@MainActor
final class TelemetryIngestClient: TelemetryBatchUploader {
    private let baseURL: URL
    private let sessionProvider: () -> SessionContext
    private let networkSession: NetworkSession
    private let decoder: JSONDecoder
    private let encoder: JSONEncoder

    init(
        baseURL: URL,
        sessionProvider: @escaping () -> SessionContext,
        networkSession: NetworkSession = URLSession.shared
    ) {
        self.baseURL = baseURL
        self.sessionProvider = sessionProvider
        self.networkSession = networkSession
        decoder = JSONDecoder()
        encoder = JSONEncoder()
    }

    func upload(batch: TelemetryBatchRequest) async throws -> TelemetryBatchUploadResponse {
        var components = URLComponents(url: baseURL, resolvingAgainstBaseURL: false)
        components?.path = "/v1/telemetry/batches"

        guard let finalURL = components?.url else {
            throw BackendError.invalidURL
        }

        var request = URLRequest(url: finalURL)
        request.httpMethod = "POST"
        request.timeoutInterval = 30
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.setValue(sessionProvider().userID.uuidString, forHTTPHeaderField: "x-user-id")
        request.httpBody = try encoder.encode(batch)

        do {
            let (data, response) = try await networkSession.data(for: request)
            guard let httpResponse = response as? HTTPURLResponse else {
                throw BackendError.invalidResponse
            }

            guard (200...299).contains(httpResponse.statusCode) else {
                let payload = try? decoder.decode(BackendErrorPayload.self, from: data)
                let message = payload?.message ?? "Telemetry upload failed with status \(httpResponse.statusCode)."
                throw BackendError.server(statusCode: httpResponse.statusCode, message: message)
            }

            do {
                return try decoder.decode(TelemetryBatchUploadResponse.self, from: data)
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
