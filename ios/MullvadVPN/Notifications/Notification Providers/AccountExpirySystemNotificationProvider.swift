//
//  AccountExpirySystemNotificationProvider.swift
//  MullvadVPN
//
//  Created by pronebird on 03/06/2021.
//  Copyright © 2021 Mullvad VPN AB. All rights reserved.
//

import Foundation
import UserNotifications

final class AccountExpirySystemNotificationProvider: NotificationProvider, SystemNotificationProvider {
    private var accountExpiry: Date?
    private var tunnelObserver: TunnelBlockObserver?

    init(tunnelManager: TunnelManager) {
        super.init()

        let tunnelObserver = TunnelBlockObserver(
            didLoadConfiguration: { [weak self] tunnelManager in
                self?.invalidate(deviceState: tunnelManager.deviceState)
            },
            didUpdateDeviceState: { [weak self] tunnelManager, deviceState, previousDeviceState in
                self?.invalidate(deviceState: deviceState)
            }
        )

        tunnelManager.addObserver(tunnelObserver)

        self.tunnelObserver = tunnelObserver
    }

    override var identifier: NotificationProviderIdentifier {
        .accountExpirySystemNotification
    }

    // MARK: - SystemNotificationProvider

    var notificationRequest: UNNotificationRequest? {
        guard let trigger else { return nil }

        _ = NSLocalizedString(
            "ACCOUNT_EXPIRY_SYSTEM_NOTIFICATION_TITLE",
            comment: "Title for system account expiry notification, fired 3 days prior to account expiry."
        )
        _ = NSLocalizedString(
            "ACCOUNT_EXPIRY_SYSTEM_NOTIFICATION_BODY",
            comment: "Message for system account expiry notification, fired 3 days prior to account expiry."
        )

        let content = UNMutableNotificationContent()
        content.title = NSString.localizedUserNotificationString(
            forKey: "ACCOUNT_EXPIRY_SYSTEM_NOTIFICATION_TITLE",
            arguments: nil
        )
        content.body = NSString.localizedUserNotificationString(
            forKey: "ACCOUNT_EXPIRY_SYSTEM_NOTIFICATION_BODY",
            arguments: nil
        )
        content.sound = UNNotificationSound.default

        return UNNotificationRequest(
            identifier: identifier.domainIdentifier,
            content: content,
            trigger: trigger
        )
    }

    var shouldRemovePendingRequests: Bool {
        // Remove pending notifications when account expiry is not set (user logged out)
        shouldRemovePendingOrDeliveredRequests
    }

    var shouldRemoveDeliveredRequests: Bool {
        // Remove delivered notifications when account expiry is not set (user logged out)
        shouldRemovePendingOrDeliveredRequests
    }

    // MARK: - Private

    private var trigger: UNNotificationTrigger? {
        guard let accountExpiry else { return nil }

        guard let triggerDate = Calendar.current.date(
            byAdding: .day,
            value: -NotificationConfiguration.closeToExpiryTriggerInterval,
            to: accountExpiry
        ) else { return nil }

        // Do not produce notification if less than 3 days left till expiry
        guard triggerDate > Date() else { return nil }

        // Create date components for calendar trigger
        let dateComponents = Calendar.current.dateComponents(
            [.second, .minute, .hour, .day, .month, .year],
            from: triggerDate
        )

        return UNCalendarNotificationTrigger(dateMatching: dateComponents, repeats: false)
    }

    private var shouldRemovePendingOrDeliveredRequests: Bool {
        accountExpiry == nil
    }

    private func invalidate(deviceState: DeviceState) {
        accountExpiry = deviceState.accountData?.expiry
        invalidate()
    }
}
