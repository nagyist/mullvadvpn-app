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
    private var task: Cancellable?

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
        task = Task {
            await logout()

            switch action {
            case .new, .existing:
                let result = await Result { [self] in
                    try await login(accountNumber: action.accountNumber)
                }
                finish(result: result.map { .some($0) })

            case .unset:
                finish(result: .success(nil))
            }
        }
    }

    override func operationDidCancel() {
        task?.cancel()
    }

    // MARK: - Private

    /**
     Logout device by performing the following steps:

     1. Delete currently logged in device from the API.
     2. Transition device state to logged out state.
     3. Remove system VPN configuration if exists.
     4. Reset tunnel status to disconnected state.

     Does nothing if device is already logged out.
     */
    private func logout() async {
        switch interactor.deviceState {
        case let .loggedIn(accountData, deviceData):
            await deleteDevice(accountNumber: accountData.number, deviceIdentifier: deviceData.identifier)
            await unsetDeviceState()

        case .revoked:
            await unsetDeviceState()

        case .loggedOut:
            break
        }
    }

    /**
     Log-in device with new or existing account by performing the following steps:

     1. Create new or retrieve existing account from the API.
     2. Store last used account number.
     3. Create new device with the API.
     4. Persist settings.
     */
    private func login(accountNumber: String?) async throws -> StoredAccountData {
        let accountData: StoredAccountData

        if let accountNumber {
            accountData = try await getAccount(accountNumber: accountNumber)
        } else {
            accountData = try await createAccount()
        }

        storeLastUsedAccount(accountNumber: accountData.number)

        let newDevice = try await createDevice(accountNumber: accountData.number)
        storeSettings(accountData: accountData, newDevice: newDevice)

        return accountData
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
    private func createAccount() async throws -> StoredAccountData {
        logger.debug("Create new account...")

        do {
            let newAccountData = try await accountsProxy.createAccount(retryStrategy: .default)

            logger.debug("Created new account.")

            return StoredAccountData(
                identifier: newAccountData.id,
                number: newAccountData.number,
                expiry: newAccountData.expiry
            )
        } catch {
            if !error.isOperationCancellationError {
                logger.error(error: error, message: "Failed to create new account.")
            }
            throw error
        }
    }

    /// Get account data from the API and produce `StoredAccountData` upon success.
    private func getAccount(accountNumber: String) async throws -> StoredAccountData {
        logger.debug("Request account data...")

        do {
            let accountData = try await accountsProxy.getAccountData(accountNumber: accountNumber, retryStrategy: .default)

            logger.debug("Received account data.")

            return StoredAccountData(
                identifier: accountData.id,
                number: accountNumber,
                expiry: accountData.expiry
            )
        } catch {
            if !error.isOperationCancellationError {
                logger.error(error: error, message: "Failed to receive account data.")
            }
            throw error
        }
    }

    /// Delete device from API.
    private func deleteDevice(accountNumber: String, deviceIdentifier: String) async {
        logger.debug("Delete current device...")

        do {
            let isDeleted = try await devicesProxy.deleteDevice(accountNumber: accountNumber, identifier: deviceIdentifier, retryStrategy: .default)

            logger.debug(isDeleted ? "Deleted device." : "Device is already deleted.")
        } catch {
            if !error.isOperationCancellationError {
                logger.error(error: error, message: "Failed to delete device.")
            }
        }
    }

    /**
     Transitions device state into logged out state by performing the following tasks:

     1. Prepare tunnel manager for removal of VPN configuration. In response tunnel manager stops processing VPN status
        notifications coming from VPN configuration.
     2. Reset device staate to logged out and persist it.
     3. Remove VPN configuration and release an instance of `Tunnel` object.
     */
    private func unsetDeviceState() async {
        // Tell the caller to unsubscribe from VPN status notifications.
        interactor.prepareForVPNConfigurationDeletion()

        // Reset tunnel and device state.
        interactor.updateTunnelStatus { tunnelStatus in
            tunnelStatus = TunnelStatus()
            tunnelStatus.state = .disconnected
        }
        interactor.setDeviceState(.loggedOut, persist: true)

        // Finish immediately if tunnel provider is not set.
        guard let tunnel = interactor.tunnel else { return }

        // Remove VPN configuration.
        do {
            try await tunnel.removeFromPreferences()
        } catch {
            // Ignore error but log it.
            logger.error(
                error: error,
                message: "Failed to remove VPN configuration."
            )
        }

        interactor.setTunnel(nil, shouldRefreshTunnelState: false)
    }

    /// Create new private key and create new device via API.
    private func createDevice(accountNumber: String) async throws -> NewDevice {
        let privateKey = PrivateKey()

        let request = REST.CreateDeviceRequest(
            publicKey: privateKey.publicKey,
            hijackDNS: false
        )

        logger.debug("Create device...")

        do {
            let newDevice = try await devicesProxy.createDevice(accountNumber: accountNumber, request: request, retryStrategy: .default)

            return NewDevice(privateKey: privateKey, device: newDevice)
        } catch {
            if !error.isOperationCancellationError {
                logger.error(error: error, message: "Failed to create device.")
            }
            throw error
        }
    }

    /// Struct that holds a private key that was used for creating a new device on the API along with the successful
    /// response from the API.
    private struct NewDevice {
        var privateKey: PrivateKey
        var device: REST.Device
    }
}
