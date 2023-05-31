//
//  SetAccountOperation.swift
//  MullvadVPN
//
//  Created by pronebird on 16/12/2021.
//  Copyright © 2021 Mullvad VPN AB. All rights reserved.
//

import Foundation
import MullvadLogging
import MullvadREST
import MullvadTypes
import Operations
import class WireGuardKitTypes.PrivateKey
import class WireGuardKitTypes.PublicKey

/**

 TODO: Store last used account!

 */

enum SetAccountAction {
    /// Set new account.
    case new

    /// Set existing account.
    case existing(String)

    /// Unset account.
    case unset

    var taskName: String {
        switch self {
        case .new:
            return "Set new account"
        case .existing:
            return "Set existing account"
        case .unset:
            return "Unset account"
        }
    }

    fileprivate var accountNumber: String? {
        switch self {
        case let .existing(accountNumber):
            return accountNumber
        case .new, .unset:
            return nil
        }
    }
}

class SetAccountOperation: ResultOperation<StoredAccountData?> {
    private let interactor: TunnelInteractor
    private let accountsProxy: REST.AccountsProxy
    private let devicesProxy: REST.DevicesProxy
    private let action: SetAccountAction

    private let logger = Logger(label: "SetAccountOperation")
    private var tasks: [Cancellable] = []

    init(
        dispatchQueue: DispatchQueue,
        interactor: TunnelInteractor,
        accountsProxy: REST.AccountsProxy,
        devicesProxy: REST.DevicesProxy,
        action: SetAccountAction
    ) {
        self.interactor = interactor
        self.accountsProxy = accountsProxy
        self.devicesProxy = devicesProxy
        self.action = action

        super.init(dispatchQueue: dispatchQueue)
    }

    // MARK: -

    override func main() {
        startLogoutFlow { [self] in
            switch action {
            case .new, .existing:
                startLoginFlow(accountNumber: action.accountNumber) { [self] result in
                    finish(result: result.map { .some($0) })
                }

            case .unset:
                finish(result: .success(nil))
            }
        }
    }

    override func operationDidCancel() {
        tasks.forEach { $0.cancel() }
        tasks.removeAll()
    }

    // MARK: - Private

    /**
     Begin logout flow by performing the following steps:

     1. Delete currently logged in device from the API if device is logged in.
     2. Transition device state to logged out state.
     3. Remove system VPN configuration if exists.
     4. Reset tunnel status to disconnected state.

     Does nothing if device is already logged out.
     */
    private func startLogoutFlow(completion: @escaping () -> Void) {
        switch interactor.deviceState {
        case let .loggedIn(accountData, deviceData):
            deleteDevice(accountNumber: accountData.number, deviceIdentifier: deviceData.identifier) { [self] error in
                unsetDeviceState(completion: completion)
            }

        case .revoked:
            unsetDeviceState(completion: completion)

        case .loggedOut:
            completion()
        }
    }

    /**
     Begin login flow with new or existing account by performing the following steps:

     1. Create new or retrieve existing account from the API.
     2. Call `didReceiveAccountData()` upon success to store last used account number and create new device, then
        persist it to settings.
     */
    private func startLoginFlow(
        accountNumber: String?,
        completion: @escaping (Result<StoredAccountData, Error>) -> Void
    ) {
        let handleResponse = { [self] (_ result: Result<StoredAccountData, Error>) in
            switch result {
            case let .success(accountData):
                continueLoginFlow(accountData, completion: completion)

            case let .failure(error):
                completion(.failure(error))
            }
        }

        if let accountNumber {
            getAccount(accountNumber: accountNumber, completion: handleResponse)
        } else {
            createAccount(completion: handleResponse)
        }
    }

    /**
     Continue login flow after receiving account data as a part of creating new or retrieving existing account from
     the API by performing the following steps:

     1. Store last used account number.
     2. Create new device with the API.
     3. Persists settings.
     */
    private func continueLoginFlow(
        _ accountData: StoredAccountData,
        completion: @escaping (Result<StoredAccountData, Error>) -> Void
    ) {
        storeLastUsedAccount(accountNumber: accountData.number)

        createDevice(accountNumber: accountData.number) { [self] result in
            if case let .success(newDevice) = result {
                storeSettings(accountData: accountData, newDevice: newDevice)
            }
            completion(result.map { _ in accountData })
        }
    }

    /// Store last used account number in settings.
    /// Errors are ignored but logged.
    private func storeLastUsedAccount(accountNumber: String) {
        logger.debug("Store last used account.")

        do {
            try SettingsManager.setLastUsedAccount(accountNumber)
        } catch {
            logger.error(error: error, message: "Failed to store last used account number.")
        }
    }

    /// Store account data and newly created device in settings and transition device state to logged in state.
    private func storeSettings(accountData: StoredAccountData, newDevice: NewDevice) {
        logger.debug("Saving settings...")

        // Create stored device data.
        let restDevice = newDevice.device
        let storedDeviceData = StoredDeviceData(
            creationDate: restDevice.created,
            identifier: restDevice.id,
            name: restDevice.name,
            hijackDNS: restDevice.hijackDNS,
            ipv4Address: restDevice.ipv4Address,
            ipv6Address: restDevice.ipv6Address,
            wgKeyData: StoredWgKeyData(
                creationDate: Date(),
                privateKey: newDevice.privateKey
            )
        )

        // Reset tunnel settings.
        interactor.setSettings(TunnelSettingsV2(), persist: true)

        // Transition device state to logged in.
        interactor.setDeviceState(.loggedIn(accountData, storedDeviceData), persist: true)
    }

    /// Create new account and produce `StoredAccountData` upon success.
    private func createAccount(completion: @escaping (Result<StoredAccountData, Error>) -> Void) {
        logger.debug("Create new account...")

        let task = accountsProxy.createAccount(retryStrategy: .default) { [self] result in
            dispatchQueue.async { [self] in
                let result = result.inspectError { error in
                    guard !error.isOperationCancellationError else { return }

                    logger.error(
                        error: error,
                        message: "Failed to create new account."
                    )
                }.map { newAccountData -> StoredAccountData in
                    logger.debug("Created new account.")

                    return StoredAccountData(
                        identifier: newAccountData.id,
                        number: newAccountData.number,
                        expiry: newAccountData.expiry
                    )
                }

                completion(result)
            }
        }

        tasks.append(task)
    }

    /// Get account data from the API and produce `StoredAccountData` upon success.
    private func getAccount(accountNumber: String, completion: @escaping (Result<StoredAccountData, Error>) -> Void) {
        logger.debug("Request account data...")

        let task = accountsProxy
            .getAccountData(accountNumber: accountNumber, retryStrategy: .default) { [self] result in
                dispatchQueue.async { [self] in
                    let result = result.inspectError { error in
                        guard !error.isOperationCancellationError else { return }

                        logger.error(error: error, message: "Failed to receive account data.")
                    }.map { accountData -> StoredAccountData in
                        logger.debug("Received account data.")

                        return StoredAccountData(
                            identifier: accountData.id,
                            number: accountNumber,
                            expiry: accountData.expiry
                        )
                    }

                    completion(result)
                }
            }

        tasks.append(task)
    }

    /// Delete device from API.
    private func deleteDevice(accountNumber: String, deviceIdentifier: String, completion: @escaping (Error?) -> Void) {
        logger.debug("Delete current device...")

        let task = devicesProxy.deleteDevice(
            accountNumber: accountNumber,
            identifier: deviceIdentifier,
            retryStrategy: .default
        ) { [self] result in
            dispatchQueue.async { [self] in
                switch result {
                case let .success(isDeleted):
                    logger.debug(isDeleted ? "Deleted device." : "Device is already deleted.")

                case let .failure(error):
                    if !error.isOperationCancellationError {
                        logger.error(error: error, message: "Failed to delete device.")
                    }
                }

                completion(result.error)
            }
        }

        tasks.append(task)
    }

    /**
     Transitions device state into logged out state by performing the following tasks:

     1. Prepare tunnel manager for removal of VPN configuration. In response tunnel manager stops processing VPN status
        notifications coming from VPN configuration.
     2. Reset device staate to logged out and persist it.
     3. Remove VPN configuration and release an instance of `Tunnel` object.
     */
    private func unsetDeviceState(completion: @escaping () -> Void) {
        // Tell the caller to unsubscribe from VPN status notifications.
        interactor.prepareForVPNConfigurationDeletion()

        // Reset tunnel and device state.
        interactor.updateTunnelStatus { tunnelStatus in
            tunnelStatus = TunnelStatus()
            tunnelStatus.state = .disconnected
        }
        interactor.setDeviceState(.loggedOut, persist: true)

        // Finish immediately if tunnel provider is not set.
        guard let tunnel = interactor.tunnel else {
            completion()
            return
        }

        // Remove VPN configuration.
        tunnel.removeFromPreferences { [self] error in
            dispatchQueue.async { [self] in
                // Ignore error but log it.
                if let error {
                    logger.error(
                        error: error,
                        message: "Failed to remove VPN configuration."
                    )
                }

                interactor.setTunnel(nil, shouldRefreshTunnelState: false)

                completion()
            }
        }
    }

    /// Create new private key and create new device via API.
    private func createDevice(accountNumber: String, completion: @escaping (Result<NewDevice, Error>) -> Void) {
        let privateKey = PrivateKey()

        let request = REST.CreateDeviceRequest(
            publicKey: privateKey.publicKey,
            hijackDNS: false
        )

        logger.debug("Create device...")

        let task = devicesProxy
            .createDevice(accountNumber: accountNumber, request: request, retryStrategy: .default) { [self] result in
                dispatchQueue.async { [self] in
                    let result = result
                        .map { device in
                            return NewDevice(privateKey: privateKey, device: device)
                        }
                        .inspectError { error in
                            logger.error(error: error, message: "Failed to create device.")
                        }

                    completion(result)
                }
            }

        tasks.append(task)
    }

    /// Struct that holds a private key that was used for creating a new device on the API along with the successful
    /// response from the API.
    private struct NewDevice {
        var privateKey: PrivateKey
        var device: REST.Device
    }
}
