import SwiftUI

@main
struct CarRanksApp: App {
    @StateObject private var environment = AppEnvironment()

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(environment)
        }
    }
}
