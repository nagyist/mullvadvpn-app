//
//  TunnelStatusNotificationProvider.swift
//  TunnelStatusNotificationProvider
//
//  Created by pronebird on 20/08/2021.
//  Copyright © 2021 Mullvad VPN AB. All rights reserved.
//

import Foundation

final class TunnelStatusNotificationProvider: NotificationProvider, InAppNotificationProvider {
    private var isWaitingForConnectivity = false
    private var noNetwork = false
    private var packetTunnelError: String?
    private var tunnelManagerError: Error?
    private var tunnelObserver: TunnelBlockObserver?

    override var identifier: NotificationProviderIdentifier {
        .tunnelStatusNotificationProvider
    }

    var notificationDescriptor: InAppNotificationDescriptor? {
        if let packetTunnelError {
            return notificationDescription(for: packetTunnelError)
        } else if let tunnelManagerError {
            return notificationDescription(for: tunnelManagerError)
        } else if isWaitingForConnectivity {
            return connectivityNotificationDescription()
        } else if noNetwork {
            return noNetworkNotificationDescription()
        } else {
            return nil
        }
    }

    init(tunnelManager: TunnelManager) {
        super.init()

        let tunnelObserver = TunnelBlockObserver(
            didLoadConfiguration: { [weak self] tunnelManager in
                self?.handleTunnelStatus(tunnelManager.tunnelStatus)
            },
            didUpdateTunnelStatus: { [weak self] tunnelManager, tunnelStatus in
                self?.handleTunnelStatus(tunnelStatus)
            },
            didFailWithError: { [weak self] tunnelManager, error in
                self?.tunnelManagerError = error
            }
        )
        self.tunnelObserver = tunnelObserver

        tunnelManager.addObserver(tunnelObserver)
    }

    // MARK: - Private

    private func handleTunnelStatus(_ tunnelStatus: TunnelStatus) {
        let invalidateForTunnelError = updateLastTunnelError(
            tunnelStatus.packetTunnelStatus.lastErrors.first?.localizedDescription
        )
        let invalidateForManagerError = updateTunnelManagerError(tunnelStatus.state)
        let invalidateForConnectivity = updateConnectivity(tunnelStatus.state)
        let invalidateForNetwork = updateNetwork(tunnelStatus.state)

        if invalidateForTunnelError || invalidateForManagerError || invalidateForConnectivity || invalidateForNetwork {
            invalidate()
        }
    }

    private func updateLastTunnelError(_ lastTunnelError: String?) -> Bool {
        if packetTunnelError != lastTunnelError {
            packetTunnelError = lastTunnelError

            return true
        }

        return false
    }

    private func updateConnectivity(_ tunnelState: TunnelState) -> Bool {
        let isWaitingState = tunnelState == .waitingForConnectivity(.noConnection)

        if isWaitingForConnectivity != isWaitingState {
            isWaitingForConnectivity = isWaitingState
            return true
        }

        return false
    }

    private func updateNetwork(_ tunnelState: TunnelState) -> Bool {
        let isWaitingState = tunnelState == .waitingForConnectivity(.noNetwork)

        if noNetwork != isWaitingState {
            noNetwork = isWaitingState
            return true
        }

        return false
    }

    private func updateTunnelManagerError(_ tunnelState: TunnelState) -> Bool {
        switch tunnelState {
        case .connecting, .connected, .reconnecting:
            // As of now, tunnel manager error can be received only when starting or stopping
            // the tunnel. Make sure to reset it on each connection attempt.
            if tunnelManagerError != nil {
                tunnelManagerError = nil
                return true
            }

        default:
            break
        }

        return false
    }

    private func notificationDescription(for packetTunnelError: String) -> InAppNotificationDescriptor {
        InAppNotificationDescriptor(
            identifier: identifier,
            style: .error,
            title: NSLocalizedString(
                "TUNNEL_LEAKING_INAPP_NOTIFICATION_TITLE",
                value: "NETWORK TRAFFIC MIGHT BE LEAKING",
                comment: ""
            ),
            body: .init(string: String(
                format: NSLocalizedString(
                    "PACKET_TUNNEL_ERROR_INAPP_NOTIFICATION_BODY",
                    value: "Could not configure VPN: %@",
                    comment: ""
                ),
                packetTunnelError
            ))
        )
    }

    private func notificationDescription(for error: Error) -> InAppNotificationDescriptor {
        let body: String

        if let startError = error as? StartTunnelError {
            body = String(
                format: NSLocalizedString(
                    "START_TUNNEL_ERROR_INAPP_NOTIFICATION_BODY",
                    value: "Failed to start the tunnel: %@.",
                    comment: ""
                ),
                startError.underlyingError?.localizedDescription ?? ""
            )
        } else if let stopError = error as? StopTunnelError {
            body = String(
                format: NSLocalizedString(
                    "STOP_TUNNEL_ERROR_INAPP_NOTIFICATION_BODY",
                    value: "Failed to stop the tunnel: %@.",
                    comment: ""
                ),
                stopError.underlyingError?.localizedDescription ?? ""
            )
        } else {
            body = error.localizedDescription
        }

        return InAppNotificationDescriptor(
            identifier: identifier,
            style: .error,
            title: NSLocalizedString(
                "TUNNEL_MANAGER_ERROR_INAPP_NOTIFICATION_TITLE",
                value: "TUNNEL ERROR",
                comment: ""
            ),
            body: .init(string: body)
        )
    }

    private func connectivityNotificationDescription() -> InAppNotificationDescriptor {
        InAppNotificationDescriptor(
            identifier: identifier,
            style: .warning,
            title: NSLocalizedString(
                "TUNNEL_NO_CONNECTIVITY_INAPP_NOTIFICATION_TITLE",
                value: "NETWORK ISSUES",
                comment: ""
            ),
            body: .init(
                string: NSLocalizedString(
                    "TUNNEL_NO_CONNECTIVITY_INAPP_NOTIFICATION_BODY",
                    value: """
                    Your device is offline. The tunnel will automatically connect once \
                    your device is back online.
                    """,
                    comment: ""
                )
            )
        )
    }

    private func noNetworkNotificationDescription() -> InAppNotificationDescriptor {
        InAppNotificationDescriptor(
            identifier: identifier,
            style: .warning,
            title: NSLocalizedString(
                "TUNNEL_NO_NETWORK_INAPP_NOTIFICATION_TITLE",
                value: "NETWORK ISSUES",
                comment: ""
            ),
            body: .init(
                string: NSLocalizedString(
                    "TUNNEL_NO_NETWORK_INAPP_NOTIFICATION_BODY",
                    value: """
                    Your device is offline. Try connecting again when the device \
                    has access to Internet.
                    """,
                    comment: ""
                )
            )
        )
    }
}
