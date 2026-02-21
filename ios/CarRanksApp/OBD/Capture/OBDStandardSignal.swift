import Foundation

/// Pollable MVP signals that map to the backend telemetry registry.
enum OBDStandardSignal: CaseIterable {
    case vehicleSpeed
    case controlModuleVoltage
    case ambientTemperature

    var command: String {
        switch self {
        case .vehicleSpeed:
            return "010D"
        case .controlModuleVoltage:
            return "0142"
        case .ambientTemperature:
            return "0146"
        }
    }

    var signalKey: String {
        switch self {
        case .vehicleSpeed:
            return "speed.vehicle"
        case .controlModuleVoltage:
            return "power.battery_voltage"
        case .ambientTemperature:
            return "environment.ambient_temp_c"
        }
    }

    var unit: String {
        switch self {
        case .vehicleSpeed:
            return "km/h"
        case .controlModuleVoltage:
            return "V"
        case .ambientTemperature:
            return "C"
        }
    }

    var sourceSignal: String {
        switch self {
        case .vehicleSpeed:
            return "01_0D"
        case .controlModuleVoltage:
            return "01_42"
        case .ambientTemperature:
            return "01_46"
        }
    }

    var confidence: Double {
        switch self {
        case .vehicleSpeed:
            return 0.99
        case .controlModuleVoltage:
            return 0.88
        case .ambientTemperature:
            return 0.80
        }
    }

    func decodeValue(from rawResponse: String) -> Double? {
        switch self {
        case .vehicleSpeed:
            return OBDResponseParser.decodeSpeedKmh(rawResponse: rawResponse)
        case .controlModuleVoltage:
            return OBDResponseParser.decodeControlModuleVoltage(rawResponse: rawResponse)
        case .ambientTemperature:
            return OBDResponseParser.decodeAmbientTemperatureC(rawResponse: rawResponse)
        }
    }
}
