import Foundation

struct SessionStoreLoadResult {
    let session: SessionContext
    let mode: DataSourceMode
}

/// Persists debug bootstrap settings so simulator iterations are stable between launches.
final class SessionContextStore {
    private enum Keys {
        static let session = "session_context"
        static let mode = "data_source_mode"
    }

    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    func load(defaultUserID: UUID, defaultVehicleUID: UUID) -> SessionStoreLoadResult {
        let fallbackSession = SessionContext(userID: defaultUserID, vehicleUID: defaultVehicleUID)

        let session: SessionContext = {
            guard let data = defaults.data(forKey: Keys.session) else { return fallbackSession }
            return (try? JSONDecoder().decode(SessionContext.self, from: data)) ?? fallbackSession
        }()

        let mode: DataSourceMode = {
            guard let raw = defaults.string(forKey: Keys.mode), let parsed = DataSourceMode(rawValue: raw) else {
                return .mock
            }
            return parsed
        }()

        return SessionStoreLoadResult(session: session, mode: mode)
    }

    func save(_ session: SessionContext, mode: DataSourceMode) {
        if let encoded = try? JSONEncoder().encode(session) {
            defaults.set(encoded, forKey: Keys.session)
        }
        defaults.set(mode.rawValue, forKey: Keys.mode)
    }
}
