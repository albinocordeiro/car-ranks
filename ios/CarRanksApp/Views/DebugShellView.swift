import SwiftUI

/// Small launcher shell so MVP slices remain discoverable while we iterate quickly.
struct DebugShellView: View {
    @ObservedObject var environment: AppEnvironment

    var body: some View {
        NavigationStack {
            List {
                Section("Session") {
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Mode: \(environment.dataSourceMode.displayName)")
                            .font(.subheadline.weight(.semibold))
                        Text("x-user-id: \(environment.sessionContext.userID.uuidString.lowercased())")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .textSelection(.enabled)
                        Text("vehicle_uid: \(environment.sessionContext.vehicleUID.uuidString.lowercased())")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .textSelection(.enabled)
                    }
                    .accessibilityIdentifier("debug-session-summary")
                }

                Section("Screens") {
                    NavigationLink("Dev Session Panel") {
                        DevSessionPanelScreen(environment: environment)
                    }
                    .accessibilityIdentifier("nav-dev-session")

                    NavigationLink("KPI Me") {
                        KpiMeScreen(environment: environment)
                    }
                    .accessibilityIdentifier("nav-kpi-me")

                    NavigationLink("KPI Charging") {
                        KpiChargingScreen(environment: environment)
                    }
                    .accessibilityIdentifier("nav-kpi-charging")

                    NavigationLink("KPI Readiness") {
                        KpiReadinessScreen(environment: environment)
                    }
                    .accessibilityIdentifier("nav-kpi-readiness")

                    NavigationLink("KPI Temperature Impact") {
                        KpiTemperatureImpactScreen(environment: environment)
                    }
                    .accessibilityIdentifier("nav-kpi-temperature-impact")

                    NavigationLink("Rankings") {
                        RankingsScreen(environment: environment)
                    }
                    .accessibilityIdentifier("nav-rankings")
                }
            }
            .navigationTitle("Car Ranks MVP")
            .accessibilityIdentifier("debug-shell-screen")
        }
    }
}
