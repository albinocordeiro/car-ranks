import Foundation

/// Controls whether live-mode screenshot runs should force scenario states
/// (empty/error) instead of depending on backend variability.
enum LiveCaptureOverrideMode: String {
    case none
    case forceStates = "force-states"
}
