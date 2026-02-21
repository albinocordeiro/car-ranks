import SwiftUI

@main
struct CarRanksApp: App {
    @StateObject private var environment = AppEnvironment()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(environment)
                .onAppear {
                    environment.telemetryUploadQueue.appDidBecomeActive()
                }
                .onChange(of: scenePhase) { _, newPhase in
                    if newPhase == .active {
                        environment.telemetryUploadQueue.appDidBecomeActive()
                    }
                }
        }
    }
}
