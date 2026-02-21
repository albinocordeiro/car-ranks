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
        return UUID(uuidString: raw ?? "") ?? UUID(uuidString: "06b3fff1-bfcc-4cda-840b-d512963bc239")!
    }

    /// Dev bootstrap UUID for vehicle_uid when a session has not been saved yet.
    static var defaultVehicleUID: UUID {
        let envValue = ProcessInfo.processInfo.environment["DEFAULT_VEHICLE_UID"]
        let plistValue = Bundle.main.object(forInfoDictionaryKey: "DEFAULT_VEHICLE_UID") as? String
        let raw = envValue ?? plistValue
        return UUID(uuidString: raw ?? "") ?? UUID(uuidString: "e11889bf-504c-4238-9583-bc8840f20e19")!
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

    /// Optional override that forces deterministic empty/error states for live capture checkpoints.
    static var liveCaptureOverrideMode: LiveCaptureOverrideMode {
        if let envMode = ProcessInfo.processInfo.environment["LIVE_CAPTURE_OVERRIDE_MODE"]?.lowercased(),
           let parsed = LiveCaptureOverrideMode(rawValue: envMode)
        {
            return parsed
        }

        let args = ProcessInfo.processInfo.arguments
        guard let index = args.firstIndex(of: "--live-capture-override"), args.indices.contains(index + 1) else {
            return .none
        }
        return LiveCaptureOverrideMode(rawValue: args[index + 1].lowercased()) ?? .none
    }
}
