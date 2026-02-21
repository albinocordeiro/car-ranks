import Foundation

/// Shared KPI atom used across multiple backend responses.
struct KpiMetric: Decodable, Equatable {
    let kpiKey: String
    let value: Double
    let unit: String
    let direction: String
    let confidenceLevel: String
    let sampleCount: Int

    private enum CodingKeys: String, CodingKey {
        case kpiKey = "kpi_key"
        case value
        case unit
        case direction
        case confidenceLevel = "confidence_level"
        case sampleCount = "sample_count"
    }
}

struct GenericKpiResponse: Decodable, Equatable {
    let vehicleUID: UUID
    let generatedAt: String
    let timeframe: String
    let temperatureBin: String
    let rankingType: String
    let kpis: [KpiMetric]

    private enum CodingKeys: String, CodingKey {
        case vehicleUID = "vehicle_uid"
        case generatedAt = "generated_at"
        case timeframe
        case temperatureBin = "temperature_bin"
        case rankingType = "ranking_type"
        case kpis
    }
}

struct ReadinessFamily: Decodable, Equatable {
    let rankingType: String
    let confidenceLevel: String
    let sampleCount: Int
    let status: String
    let missingRequirements: [String]

    private enum CodingKeys: String, CodingKey {
        case rankingType = "ranking_type"
        case confidenceLevel = "confidence_level"
        case sampleCount = "sample_count"
        case status
        case missingRequirements = "missing_requirements"
    }
}

struct ReadinessResponse: Decodable, Equatable {
    let vehicleUID: UUID
    let generatedAt: String
    let timeframe: String
    let families: [ReadinessFamily]

    private enum CodingKeys: String, CodingKey {
        case vehicleUID = "vehicle_uid"
        case generatedAt = "generated_at"
        case timeframe
        case families
    }
}

struct TemperatureImpactResponse: Decodable, Equatable {
    struct CohortBenchmark: Decodable, Equatable {
        struct Percentiles: Decodable, Equatable {
            let coldWeatherRangeRetention: Double?
            let rangeTemperatureSensitivityIndex: Double?
            let coldWeatherChargeSpeedRetention: Double?

            private enum CodingKeys: String, CodingKey {
                case coldWeatherRangeRetention = "cold_weather_range_retention"
                case rangeTemperatureSensitivityIndex = "range_temperature_sensitivity_index"
                case coldWeatherChargeSpeedRetention = "cold_weather_charge_speed_retention"
            }
        }

        let cohortSize: Int
        let percentiles: Percentiles

        private enum CodingKeys: String, CodingKey {
            case cohortSize = "cohort_size"
            case percentiles
        }
    }

    let vehicleUID: UUID
    let generatedAt: String
    let baselineTemperatureBin: String
    let compareTemperatureBin: String
    let metrics: [KpiMetric]
    let cohortBenchmark: CohortBenchmark

    private enum CodingKeys: String, CodingKey {
        case vehicleUID = "vehicle_uid"
        case generatedAt = "generated_at"
        case baselineTemperatureBin = "baseline_temperature_bin"
        case compareTemperatureBin = "compare_temperature_bin"
        case metrics
        case cohortBenchmark = "cohort_benchmark"
    }
}

struct RankingsResponse: Decodable, Equatable {
    struct Filters: Decodable, Equatable {
        let powertrainClass: String?
        let make: String?
        let model: String?
        let trim: String?
        let yearBand: String?
        let region: String?

        private enum CodingKeys: String, CodingKey {
            case powertrainClass = "powertrain_class"
            case make
            case model
            case trim
            case yearBand = "year_band"
            case region
        }
    }

    struct Cohort: Decodable, Equatable {
        let cohortKey: String
        let cohortSize: Int
        let sampleGatePassed: Bool

        private enum CodingKeys: String, CodingKey {
            case cohortKey = "cohort_key"
            case cohortSize = "cohort_size"
            case sampleGatePassed = "sample_gate_passed"
        }
    }

    struct Row: Decodable, Equatable {
        let rank: Int
        let vehicleUID: UUID
        let score: Double
        let confidenceLevel: String
        let kpis: [String: Double]

        private enum CodingKeys: String, CodingKey {
            case rank
            case vehicleUID = "vehicle_uid"
            case score
            case confidenceLevel = "confidence_level"
            case kpis
        }
    }

    struct Page: Decodable, Equatable {
        let limit: Int
        let offset: Int
        let hasMore: Bool

        private enum CodingKeys: String, CodingKey {
            case limit
            case offset
            case hasMore = "has_more"
        }
    }

    let generatedAt: String
    let rankingType: String
    let timeframe: String
    let temperatureBin: String
    let filters: Filters
    let cohort: Cohort
    let rows: [Row]
    let page: Page

    private enum CodingKeys: String, CodingKey {
        case generatedAt = "generated_at"
        case rankingType = "ranking_type"
        case timeframe
        case temperatureBin = "temperature_bin"
        case filters
        case cohort
        case rows
        case page
    }
}

struct BackendErrorPayload: Decodable {
    let error: String
    let message: String
}
