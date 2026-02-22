import XCTest
@testable import CarRanksApp

final class OBDErrorPresentationTests: XCTestCase {
    func testTransportErrorUsesRawMessageWithoutNetworkPrefix() {
        let error = BackendError.transport("Adapter command ATZ failed with '?'.")
        XCTAssertEqual(
            OBDErrorPresentation.message(from: error),
            "Adapter command ATZ failed with '?'."
        )
    }

    func testDecodeErrorUsesAdapterDecodeMessage() {
        let error = BackendError.decode("malformed payload")
        XCTAssertEqual(
            OBDErrorPresentation.message(from: error),
            "Failed to decode adapter response: malformed payload"
        )
    }
}
