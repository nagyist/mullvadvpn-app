//
//  CustomActorExecutor.swift
//  MullvadVPNTests
//
//  Created by pronebird on 02/06/2023.
//  Copyright © 2023 Mullvad VPN AB. All rights reserved.
//

import XCTest

let GoogleURL = URL(string: "https://google.com/")!

final class CustomActorExecutor: XCTestCase {

    func testTraditionalApproach() {
        let e = expectation(description: "Wait for download")

        let task = URLSession.shared.dataTask(with: GoogleURL) { data, response, error in
            if let error {
                print(error)
            }
            e.fulfill()
        }
        task.resume()

        waitForExpectations(timeout: 30000)
    }


    func testSwiftConcurrency() async throws {
        let (data, response) = try await withCheckedThrowingContinuation { (_ continuation: Continuation) in
            let task = URLSession.shared.dataTask(with: GoogleURL) { data, response, error in
                if let error {
                    continuation.resume(throwing: error)
                } else {
                    let value = (data!, response!)
                    continuation.resume(returning: value)
                }
            }
            task.resume()
        }

        print(data)
        print(response)

        let (data2, response2) = try await URLSession.shared.data(from: GoogleURL)

        print(data2)
        print(response2)
    }


    @MyCustomActor
    func testSwiftConcurrencyWithCustomActor() async throws {
        let (data, response) = try await URLSession.shared.data(from: GoogleURL)

        print(data)
        print(response)

        let (data2, response2) = try await URLSession.shared.data(from: GoogleURL)

        print(data2)
        print(response2)
    }

}


private let executorQueue = DispatchQueue(label: "MyCustomExecutorQueue")

final class CustomSerialExecutor: SerialExecutor {
    private static let sharedExecutor = CustomSerialExecutor()

    static var sharedUnownedExecutor: UnownedSerialExecutor {
        return sharedExecutor.asUnownedSerialExecutor()
    }

    func enqueue(_ job: consuming ExecutorJob) {
        let unownedJob = UnownedJob(job)

        executorQueue.async {
            unownedJob.runSynchronously(on: self.asUnownedSerialExecutor())
        }
    }

    func asUnownedSerialExecutor() -> UnownedSerialExecutor {
        UnownedSerialExecutor(ordinary: self)
    }
}

actor Worker {
    nonisolated var unownedExecutor: UnownedSerialExecutor {
        return CustomSerialExecutor.sharedUnownedExecutor
    }
}

@globalActor
final actor MyCustomActor {
    static var shared = Worker()
}

typealias Respose = (Data, URLResponse)
typealias Continuation = CheckedContinuation<Respose, Error>
