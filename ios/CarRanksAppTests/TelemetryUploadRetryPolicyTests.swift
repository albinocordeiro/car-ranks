import XCTest
@testable import CarRanksApp

final class TelemetryUploadRetryPolicyTests: XCTestCase {
    func testRetryableErrorsMatchTransientFailureClasses() {
        let policy = TelemetryUploadRetryPolicy.standard

        XCTAssertTrue(policy.shouldRetry(error: .transport("offline"), retryCount: 0))
        XCTAssertTrue(policy.shouldRetry(error: .invalidResponse, retryCount: 0))
        XCTAssertTrue(policy.shouldRetry(error: .server(statusCode: 503, message: "down"), retryCount: 0))
        XCTAssertTrue(policy.shouldRetry(error: .server(statusCode: 429, message: "rate limited"), retryCount: 0))

        XCTAssertFalse(policy.shouldRetry(error: .invalidURL, retryCount: 0))
        XCTAssertFalse(policy.shouldRetry(error: .decode("bad payload"), retryCount: 0))
        XCTAssertFalse(policy.shouldRetry(error: .server(statusCode: 401, message: "unauthorized"), retryCount: 0))
    }

    func testRetryBackoffIsCapped() {
        let policy = TelemetryUploadRetryPolicy(
            maxAttempts: 6,
            initialDelaySeconds: 2,
            maxDelaySeconds: 10,
            backoffMultiplier: 2
        )

        XCTAssertEqual(policy.delaySeconds(forRetryCount: 1), 2, accuracy: 0.0001)
        XCTAssertEqual(policy.delaySeconds(forRetryCount: 2), 4, accuracy: 0.0001)
        XCTAssertEqual(policy.delaySeconds(forRetryCount: 3), 8, accuracy: 0.0001)
        XCTAssertEqual(policy.delaySeconds(forRetryCount: 4), 10, accuracy: 0.0001)
        XCTAssertEqual(policy.delaySeconds(forRetryCount: 6), 10, accuracy: 0.0001)
    }
}
