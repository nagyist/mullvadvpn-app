package net.mullvad.mullvadvpn.compose.theme.dimensions

import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

data class Dimensions(
    val cellEndPadding: Dp = 16.dp,
    val cellFooterTopPadding: Dp = 6.dp,
    val cellHeight: Dp = 52.dp,
    val cellLabelVerticalPadding: Dp = 14.dp,
    val cellStartPadding: Dp = 22.dp,
    val cityRowPadding: Dp = 34.dp,
    val countryRowPadding: Dp = 18.dp,
    val expandableCellChevronSize: Dp = 30.dp,
    val indentedCellStartPadding: Dp = 38.dp,
    val infoButtonVerticalPadding: Dp = 13.dp,
    val listIconSize: Dp = 24.dp,
    val listItemDivider: Dp = 1.dp,
    val listItemHeight: Dp = 50.dp,
    val listItemHeightExtra: Dp = 60.dp,
    val loadingSpinnerPadding: Dp = 12.dp,
    val loadingSpinnerSize: Dp = 24.dp,
    val loadingSpinnerStrokeWidth: Dp = 3.dp,
    val mediumPadding: Dp = 16.dp,
    val progressIndicatorSize: Dp = 60.dp,
    val relayCircleSize: Dp = 16.dp,
    val relayRowPadding: Dp = 50.dp,
    val selectableCellTextMargin: Dp = 12.dp,
    val sideMargin: Dp = 22.dp,
    val smallPadding: Dp = 8.dp,
    val verticalSpace: Dp = 20.dp
)

val defaultDimensions = Dimensions()
// Add more configurations here if needed
