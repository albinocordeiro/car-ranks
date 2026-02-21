import Foundation

/// Reads static runtime configuration from Info.plist with environment overrides.
/// The env override path makes simulator automation deterministic during checkpoints.
enum AppEnvironmentConfig {
    /// Resolve API base URL with an env-first policy so simulator scripts can pivot quickly.
    static var apiBaseURL: URL {
        let envValue = ProcessInfo.processInfo.environment["API_BASE_URL"]
        let plistValue = Bundle.main.object(forInfoDictionaryKey: "API_BASE_URL") as? String
        let raw = envValue ?? plistValue ?? "http://127.0.0.1:8080"
        guard let url = URL(string: raw), let scheme = url.scheme else {
            return URL(string: "http://127.0.0.1:8080")!
        }

        // Simulator default loopback should use host machine route alias.
        if scheme == "http" || scheme == "https" {
            return url
        }
        return URL(string: "http://127.0.0.1:8080")!
    }

    /// Dev bootstrap UUID for x-user-id when a session has not been saved yet.
    static var defaultUserID: UUID {
        let envValue = ProcessInfo.processInfo.environment["DEFAULT_X_USER_ID"]
        let plistValue = Bundle.main.object(forInfoDictionaryKey: "DEFAULT_X_USER_ID") as? String
        let raw = envValue ?? plistValue
        return UUID(uuidString: raw ?? "") ?? UUID(uuidString: "54e9b082-226f-4b7b-b98b-703832c5dfb2")!
    }

    /// Dev bootstrap UUID for vehicle_uid when a session has not been saved yet.
    static var defaultVehicleUID: UUID {
        let envValue = ProcessInfo.processInfo.environment["DEFAULT_VEHICLE_UID"]
        let plistValue = Bundle.main.object(forInfoDictionaryKey: "DEFAULT_VEHICLE_UID") as? String
        let raw = envValue ?? plistValue
        return UUID(uuidString: raw ?? "") ?? UUID(uuidString: "5bf1cb17-a0c6-404b-81f6-c407b80ea3b4")!
    }

    /// Optional launch override used by UI tests and screenshot scripts.
    static var launchDataSourceMode: DataSourceMode? {
        if let envMode = ProcessInfo.processInfo.environment["DATA_SOURCE_MODE"]?.lowercased(),
           let parsed = DataSourceMode(rawValue: envMode)
        {
            return parsed
        }

        let args = ProcessInfo.processInfo.arguments
        guard let index = args.firstIndex(of: "--data-source-mode"), args.indices.contains(index + 1) else {
            return nil
        }
        return DataSourceMode(rawValue: args[index + 1].lowercased())
    }
}
