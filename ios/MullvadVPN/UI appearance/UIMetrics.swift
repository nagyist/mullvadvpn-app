//
//  UIMetrics.swift
//  MullvadVPN
//
//  Created by pronebird on 10/03/2021.
//  Copyright © 2021 Mullvad VPN AB. All rights reserved.
//

import UIKit

enum UIMetrics {
    enum CustomAlert {
        /// Layout margins for container (main view) in `CustomAlertViewController`
        static let containerMargins = NSDirectionalEdgeInsets(
            top: 28,
            leading: 16,
            bottom: 16,
            trailing: 16
        )

        /// Spacing between views in container (main view) in `CustomAlertViewController`
        static let containerSpacing: CGFloat = 16
    }

    enum DimmingView {
        static let opacity: CGFloat = 0.5
        static let cornerRadius: CGFloat = 8
        static let backgroundColor: UIColor = .black
    }

    enum FormSheetTransition {
        static let duration: TimeInterval = 0.5
        static let delay: TimeInterval = .zero
        static let animationOptions: UIView.AnimationOptions = [.curveEaseInOut]
    }

    enum RedeemVoucher {
        static let cornerRadius = 8.0
        static let preferredContentSize = CGSize(width: 292, height: 263)
    }
}

extension UIMetrics {
    /// Common layout margins for content presentation
    static let contentLayoutMargins = NSDirectionalEdgeInsets(top: 24, leading: 24, bottom: 24, trailing: 24)

    /// Common content margins for content presentation
    static let contentInsets = UIEdgeInsets(top: 24, left: 24, bottom: 24, right: 24)

    /// Common layout margins for row views presentation
    /// Similar to `settingsCellLayoutMargins` however maintains equal horizontal spacing
    static let rowViewLayoutMargins = NSDirectionalEdgeInsets(top: 16, leading: 24, bottom: 16, trailing: 24)

    /// Common layout margins for settings cell presentation
    static let settingsCellLayoutMargins = NSDirectionalEdgeInsets(top: 16, leading: 24, bottom: 16, trailing: 12)

    /// Common layout margins for text field in settings input cell presentation
    static let settingsInputCellTextFieldLayoutMargins = UIEdgeInsets(
        top: 0,
        left: 8,
        bottom: 0,
        right: 8
    )

    /// Common layout margins for location cell presentation
    static let selectLocationCellLayoutMargins = NSDirectionalEdgeInsets(top: 16, leading: 28, bottom: 16, trailing: 12)

    /// Common cell indentation width
    static let cellIndentationWidth: CGFloat = 16

    /// Group of constants related to in-app notifications banner.
    enum InAppBannerNotification {
        /// Layout margins for contents presented within the banner.
        static let layoutMargins = NSDirectionalEdgeInsets(top: 16, leading: 24, bottom: 16, trailing: 24)

        /// Size of little round severity indicator.
        static let indicatorSize = CGSize(width: 12, height: 12)
    }

    /// Spacing used in stack views of buttons
    static let interButtonSpacing: CGFloat = 16

    /// Spacing used between distinct sections of views
    static let sectionSpacing: CGFloat = 24

    /// Text field margins
    static let textFieldMargins = UIEdgeInsets(top: 12, left: 14, bottom: 12, right: 14)

    /// Corner radius used for controls such as buttons and text fields
    static let controlCornerRadius: CGFloat = 4

    /// Maximum width of the split view content container on iPad
    static let maximumSplitViewContentContainerWidth: CGFloat = 810 * 0.7

    /// Minimum sidebar width in points
    static let minimumSplitViewSidebarWidth: CGFloat = 300

    /// Maximum sidebar width in percentage points (0...1)
    static let maximumSplitViewSidebarWidthFraction: CGFloat = 0.3

    /// Spacing between buttons in header bar.
    static let headerBarButtonSpacing: CGFloat = 20

    /// Size of a square logo image in header bar.
    static let headerBarLogoSize: CGFloat = 44

    /// Height of brand name. Width is automatically produced based on aspect ratio.
    static let headerBarBrandNameHeight: CGFloat = 18
}
