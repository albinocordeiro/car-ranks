import Foundation
import Combine

@MainActor
final class AppEnvironment: ObservableObject {
    @Published var sessionContext: SessionContext
    @Published var dataSourceMode: DataSourceMode

    private let sessionStore: SessionContextStore
    private let baseURL: URL
    private let captureScenario: CaptureScenario

    init(
        sessionStore: SessionContextStore = SessionContextStore(),
        baseURL: URL = AppEnvironmentConfig.apiBaseURL,
        captureScenario: CaptureScenario = CaptureScenario.current()
    ) {
        self.sessionStore = sessionStore
        self.baseURL = baseURL
        self.captureScenario = captureScenario

        let loaded = sessionStore.load(defaultUserID: AppEnvironmentConfig.defaultUserID, defaultVehicleUID: AppEnvironmentConfig.defaultVehicleUID)
        sessionContext = loaded.session
        if let launchMode = AppEnvironmentConfig.launchDataSourceMode {
            dataSourceMode = launchMode
        } else if captureScenario == .none {
            dataSourceMode = loaded.mode
        } else {
            // Capture/test runs default to mock mode unless a launch override is explicitly set.
            dataSourceMode = .mock
        }
    }

    func saveSession(userIDString: String, vehicleUIDString: String, mode: DataSourceMode) -> Bool {
        guard let userID = UUID(uuidString: userIDString), let vehicleUID = UUID(uuidString: vehicleUIDString) else {
            return false
        }
        sessionContext = SessionContext(userID: userID, vehicleUID: vehicleUID)
        dataSourceMode = mode
        sessionStore.save(sessionContext, mode: mode)
        return true
    }

    func makeBackendClient() -> BackendClient {
        switch dataSourceMode {
        case .mock:
            return MockBackendClient(captureScenario: captureScenario)
        case .live:
            return LiveBackendClient(
                baseURL: baseURL,
                sessionProvider: { [weak self] in
                    guard let self else {
                        return SessionContext(userID: AppEnvironmentConfig.defaultUserID, vehicleUID: AppEnvironmentConfig.defaultVehicleUID)
                    }
                    return self.sessionContext
                }
            )
        }
    }

    var activeCaptureScenario: CaptureScenario {
        captureScenario
    }
}
