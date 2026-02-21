import SwiftUI

struct DevSessionPanelScreen: View {
    @ObservedObject private var environment: AppEnvironment
    @StateObject private var viewModel: DevSessionPanelViewModel

    init(environment: AppEnvironment, onSaved: @escaping () -> Void = {}) {
        self.environment = environment
        _viewModel = StateObject(
            wrappedValue: DevSessionPanelViewModel(
                session: environment.sessionContext,
                mode: environment.dataSourceMode,
                saveHandler: { userID, vehicleUID, mode in
                    environment.saveSession(userIDString: userID, vehicleUIDString: vehicleUID, mode: mode)
                },
                onSaved: onSaved
            )
        )
    }

    var body: some View {
        DevSessionPanelView(viewModel: viewModel)
            .navigationTitle("Dev Session")
            .navigationBarTitleDisplayMode(.inline)
    }
}

struct DevSessionPanelView: View {
    @ObservedObject var viewModel: DevSessionPanelViewModel

    var body: some View {
        Form {
            Section("Identity") {
                TextField("x-user-id UUID", text: $viewModel.userIDInput)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled(true)
                    .font(.system(.footnote, design: .monospaced))
                    .accessibilityIdentifier("session-user-id-field")

                TextField("vehicle_uid UUID", text: $viewModel.vehicleUIDInput)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled(true)
                    .font(.system(.footnote, design: .monospaced))
                    .accessibilityIdentifier("session-vehicle-uid-field")
            }

            Section("Data Source") {
                Picker("Mode", selection: $viewModel.selectedMode) {
                    ForEach(DataSourceMode.allCases) { mode in
                        Text(mode.displayName).tag(mode)
                    }
                }
                .pickerStyle(.segmented)
                .accessibilityIdentifier("session-mode-picker")
            }

            Section {
                Button("Save Session") {
                    viewModel.save()
                }
                .buttonStyle(.borderedProminent)
                .frame(maxWidth: .infinity, alignment: .center)
                .accessibilityIdentifier("session-save-button")
            }

            if let statusMessage = viewModel.statusMessage {
                Section("Status") {
                    Text(statusMessage)
                        .foregroundStyle(viewModel.hasError ? .red : .green)
                        .accessibilityIdentifier("session-save-status")
                }
            }
        }
        .accessibilityIdentifier("dev-session-panel-screen")
    }
}
