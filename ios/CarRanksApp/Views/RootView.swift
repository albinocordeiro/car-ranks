import SwiftUI

struct RootView: View {
    @EnvironmentObject private var environment: AppEnvironment

    var body: some View {
        switch environment.activeCaptureScenario {
        case .none:
            DebugShellView(environment: environment)
        case .devSession:
            NavigationStack {
                DevSessionPanelScreen(environment: environment)
            }
        case .kpiMeLoading, .kpiMeSuccess, .kpiMeEmpty, .kpiMeError, .kpiMeErrorThenSuccess:
            NavigationStack {
                KpiMeScreen(environment: environment)
            }
        case .kpiChargingLoading, .kpiChargingSuccess, .kpiChargingEmpty, .kpiChargingError:
            NavigationStack {
                KpiChargingScreen(environment: environment)
            }
        case .kpiReadinessLoading, .kpiReadinessSuccess, .kpiReadinessEmpty, .kpiReadinessError:
            NavigationStack {
                KpiReadinessScreen(environment: environment)
            }
        case .kpiTemperatureImpactLoading, .kpiTemperatureImpactSuccess, .kpiTemperatureImpactEmpty, .kpiTemperatureImpactError:
            NavigationStack {
                KpiTemperatureImpactScreen(environment: environment)
            }
        case .rankingsLoading, .rankingsSuccess, .rankingsEmpty, .rankingsError:
            NavigationStack {
                RankingsScreen(environment: environment)
            }
        }
    }
}
