import Foundation

/// Controls deterministic UI scenarios used by simulator screenshot automation and UI tests.
enum CaptureScenario: String {
    case none
    case kpiMeLoading = "kpi-me-loading"
    case kpiMeSuccess = "kpi-me-success"
    case kpiMeEmpty = "kpi-me-empty"
    case kpiMeError = "kpi-me-error"
    case kpiMeErrorThenSuccess = "kpi-me-error-then-success"
    case kpiChargingLoading = "kpi-charging-loading"
    case kpiChargingSuccess = "kpi-charging-success"
    case kpiChargingEmpty = "kpi-charging-empty"
    case kpiChargingError = "kpi-charging-error"
    case kpiReadinessLoading = "kpi-readiness-loading"
    case kpiReadinessSuccess = "kpi-readiness-success"
    case kpiReadinessEmpty = "kpi-readiness-empty"
    case kpiReadinessError = "kpi-readiness-error"
    case kpiTemperatureImpactLoading = "kpi-temperature-impact-loading"
    case kpiTemperatureImpactSuccess = "kpi-temperature-impact-success"
    case kpiTemperatureImpactEmpty = "kpi-temperature-impact-empty"
    case kpiTemperatureImpactError = "kpi-temperature-impact-error"
    case rankingsLoading = "rankings-loading"
    case rankingsSuccess = "rankings-success"
    case rankingsEmpty = "rankings-empty"
    case rankingsError = "rankings-error"
    case devSession = "dev-session"

    static func current() -> CaptureScenario {
        let env = ProcessInfo.processInfo.environment["CAPTURE_SCENARIO"]
        if let env, let parsed = CaptureScenario(rawValue: env) {
            return parsed
        }

        let args = ProcessInfo.processInfo.arguments
        guard let index = args.firstIndex(of: "--capture-scenario"), args.indices.contains(index + 1) else {
            return .none
        }
        let value = args[index + 1]
        return CaptureScenario(rawValue: value) ?? .none
    }
}
