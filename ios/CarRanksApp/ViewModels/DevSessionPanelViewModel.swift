import Foundation
import Combine

@MainActor
final class DevSessionPanelViewModel: ObservableObject {
    @Published var userIDInput: String
    @Published var vehicleUIDInput: String
    @Published var selectedMode: DataSourceMode
    @Published private(set) var statusMessage: String?
    @Published private(set) var hasError = false

    private let saveHandler: (String, String, DataSourceMode) -> Bool
    private let onSaved: () -> Void

    init(
        session: SessionContext,
        mode: DataSourceMode,
        saveHandler: @escaping (String, String, DataSourceMode) -> Bool,
        onSaved: @escaping () -> Void = {}
    ) {
        userIDInput = session.userID.uuidString.lowercased()
        vehicleUIDInput = session.vehicleUID.uuidString.lowercased()
        selectedMode = mode
        self.saveHandler = saveHandler
        self.onSaved = onSaved
    }

    /// Validates and persists a debug session payload that mirrors backend auth linkage.
    func save() {
        let didSave = saveHandler(userIDInput.trimmingCharacters(in: .whitespacesAndNewlines), vehicleUIDInput.trimmingCharacters(in: .whitespacesAndNewlines), selectedMode)
        if didSave {
            hasError = false
            statusMessage = "Saved. KPI screen will use this session."
            onSaved()
        } else {
            hasError = true
            statusMessage = "Invalid UUID input. Please correct both fields."
        }
    }
}
