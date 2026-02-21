import XCTest
@testable import CarRanksApp

final class AppEnvironmentConfigTests: XCTestCase {
    func testLiveCaptureOverrideModeDefaultsToNone() {
        unsetenv("LIVE_CAPTURE_OVERRIDE_MODE")
        XCTAssertEqual(AppEnvironmentConfig.liveCaptureOverrideMode, .none)
    }

    func testLiveCaptureOverrideModeReadsEnvironmentOverride() {
        setenv("LIVE_CAPTURE_OVERRIDE_MODE", "force-states", 1)
        defer { unsetenv("LIVE_CAPTURE_OVERRIDE_MODE") }
        XCTAssertEqual(AppEnvironmentConfig.liveCaptureOverrideMode, .forceStates)
    }
}
