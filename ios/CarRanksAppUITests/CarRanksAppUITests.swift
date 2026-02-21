import XCTest

@MainActor
final class CarRanksAppUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    func testKpiMeLoadingStateRenders() {
        assertCaptureScenario("kpi-me-loading", markerText: "Loading KPI snapshot...")
    }

    func testKpiMeSuccessStateRenders() {
        assertCaptureScenario("kpi-me-success", markerText: "Ev Net Energy Efficiency")
    }

    func testKpiMeEmptyStateRenders() {
        assertCaptureScenario("kpi-me-empty", markerText: "No KPI Data")
    }

    func testKpiMeErrorStateRenders() {
        assertCaptureScenario("kpi-me-error", markerText: "Failed to load KPI snapshot")
    }

    func testKpiChargingLoadingStateRenders() {
        assertCaptureScenario("kpi-charging-loading", markerText: "Loading charging KPI snapshot...")
    }

    func testKpiChargingSuccessStateRenders() {
        assertCaptureScenario("kpi-charging-success", markerText: "Temp Adjusted Charge Acceptance Score")
    }

    func testKpiChargingEmptyStateRenders() {
        assertCaptureScenario("kpi-charging-empty", markerText: "No Charging KPI Data")
    }

    func testKpiChargingErrorStateRenders() {
        assertCaptureScenario("kpi-charging-error", markerText: "Failed to load charging KPI snapshot")
    }

    func testKpiReadinessLoadingStateRenders() {
        assertCaptureScenario("kpi-readiness-loading", markerText: "Loading readiness status...")
    }

    func testKpiReadinessSuccessStateRenders() {
        assertCaptureScenario("kpi-readiness-success", markerText: "Ev Range Efficiency")
    }

    func testKpiReadinessEmptyStateRenders() {
        assertCaptureScenario("kpi-readiness-empty", markerText: "No Readiness Data")
    }

    func testKpiReadinessErrorStateRenders() {
        assertCaptureScenario("kpi-readiness-error", markerText: "Failed to load readiness status")
    }

    func testKpiTemperatureImpactLoadingStateRenders() {
        assertCaptureScenario("kpi-temperature-impact-loading", markerText: "Loading temperature impact...")
    }

    func testKpiTemperatureImpactSuccessStateRenders() {
        assertCaptureScenario("kpi-temperature-impact-success", markerText: "Cold Weather Range Retention")
    }

    func testKpiTemperatureImpactEmptyStateRenders() {
        assertCaptureScenario("kpi-temperature-impact-empty", markerText: "No Temperature Impact Data")
    }

    func testKpiTemperatureImpactErrorStateRenders() {
        assertCaptureScenario("kpi-temperature-impact-error", markerText: "Failed to load temperature impact")
    }

    func testRankingsLoadingStateRenders() {
        assertCaptureScenario("rankings-loading", markerText: "Loading rankings...")
    }

    func testRankingsSuccessStateRenders() {
        assertCaptureScenario("rankings-success", markerText: "Type: ev_temperature_impact")
    }

    func testRankingsEmptyStateRenders() {
        assertCaptureScenario("rankings-empty", markerText: "No Rankings Data")
    }

    func testRankingsErrorStateRenders() {
        assertCaptureScenario("rankings-error", markerText: "Failed to load rankings")
    }

    func testDevSessionPanelSavesAndKpiScreenLoads() {
        let app = launchApp(captureScenario: nil, mode: "mock")

        XCTAssertTrue(waitForElement(identifier: "debug-shell-screen", in: app, timeout: 5))
        app.cells.containing(.staticText, identifier: "Dev Session Panel").firstMatch.tap()

        XCTAssertTrue(waitForElement(identifier: "dev-session-panel-screen", in: app, timeout: 5))
        replaceText(in: app.textFields["session-user-id-field"], with: "e67d9de7-76a4-4f5f-a388-f40ce648b474")
        replaceText(in: app.textFields["session-vehicle-uid-field"], with: "f6d8e2c8-c755-4d7e-88bc-c3ad5f8258c3")
        app.buttons["Mock"].tap()

        app.buttons["session-save-button"].tap()
        XCTAssertTrue(waitForElement(identifier: "session-save-status", in: app, timeout: 4))

        let backButton = app.navigationBars.buttons.firstMatch
        XCTAssertTrue(backButton.waitForExistence(timeout: 2))
        backButton.tap()

        app.cells.containing(.staticText, identifier: "KPI Me").firstMatch.tap()
        XCTAssertTrue(waitForStaticText("Ev Net Energy Efficiency", in: app, timeout: 10))
    }

    func testRetryTransitionsFromErrorToSuccess() {
        let app = launchApp(captureScenario: "kpi-me-error-then-success", mode: "mock")

        XCTAssertTrue(waitForStaticText("Failed to load KPI snapshot", in: app, timeout: 6))
        app.buttons["Retry"].tap()
        XCTAssertTrue(waitForStaticText("Ev Net Energy Efficiency", in: app, timeout: 10))
    }

    @discardableResult
    private func launchApp(captureScenario: String?, mode: String) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["DATA_SOURCE_MODE"] = mode
        if let captureScenario {
            app.launchEnvironment["CAPTURE_SCENARIO"] = captureScenario
        }
        app.launch()
        return app
    }

    private func waitForElement(identifier: String, in app: XCUIApplication, timeout: TimeInterval) -> Bool {
        let query = app.descendants(matching: .any).matching(identifier: identifier)
        return query.firstMatch.waitForExistence(timeout: timeout)
    }

    private func waitForStaticText(_ text: String, in app: XCUIApplication, timeout: TimeInterval) -> Bool {
        app.staticTexts[text].firstMatch.waitForExistence(timeout: timeout)
    }

    private func assertCaptureScenario(_ scenario: String, markerText: String) {
        let app = launchApp(captureScenario: scenario, mode: "mock")
        XCTAssertTrue(waitForStaticText(markerText, in: app, timeout: 8), "Missing state marker: \(markerText)")
        app.terminate()
    }

    private func replaceText(in element: XCUIElement, with text: String) {
        XCTAssertTrue(element.waitForExistence(timeout: 5))
        element.tap()

        if let currentValue = element.value as? String {
            let clearSequence = String(repeating: XCUIKeyboardKey.delete.rawValue, count: currentValue.count)
            element.typeText(clearSequence)
        }

        element.typeText(text)
    }
}
