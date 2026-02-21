import XCTest
@testable import CarRanksApp

final class OBDReconnectPolicyTests: XCTestCase {
    func testShouldRetryStopsAfterMaxAttempts() {
        let policy = OBDReconnectPolicy(
            maxAttempts: 3,
            initialDelaySeconds: 1,
            maxDelaySeconds: 8,
            backoffMultiplier: 2
        )

        XCTAssertTrue(policy.shouldRetry(attempt: 1))
        XCTAssertTrue(policy.shouldRetry(attempt: 2))
        XCTAssertTrue(policy.shouldRetry(attempt: 3))
        XCTAssertFalse(policy.shouldRetry(attempt: 4))
    }

    func testDelayUsesCappedExponentialBackoff() {
        let policy = OBDReconnectPolicy(
            maxAttempts: 6,
            initialDelaySeconds: 0.5,
            maxDelaySeconds: 3,
            backoffMultiplier: 2
        )

        XCTAssertEqual(policy.delaySeconds(forAttempt: 1), 0.5, accuracy: 0.0001)
        XCTAssertEqual(policy.delaySeconds(forAttempt: 2), 1.0, accuracy: 0.0001)
        XCTAssertEqual(policy.delaySeconds(forAttempt: 3), 2.0, accuracy: 0.0001)
        XCTAssertEqual(policy.delaySeconds(forAttempt: 4), 3.0, accuracy: 0.0001) // capped
        XCTAssertEqual(policy.delaySeconds(forAttempt: 6), 3.0, accuracy: 0.0001) // capped
    }
}
