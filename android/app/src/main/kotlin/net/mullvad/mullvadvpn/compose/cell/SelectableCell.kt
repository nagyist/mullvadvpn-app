package net.mullvad.mullvadvpn.compose.cell

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.Dp
import net.mullvad.mullvadvpn.R
import net.mullvad.mullvadvpn.compose.component.SpacedColumn
import net.mullvad.mullvadvpn.compose.theme.AlphaInvisible
import net.mullvad.mullvadvpn.compose.theme.AlphaVisible
import net.mullvad.mullvadvpn.compose.theme.AppTheme
import net.mullvad.mullvadvpn.compose.theme.Dimens

@Preview
@Composable
private fun PreviewSelectableCell() {
    AppTheme {
        SpacedColumn(Modifier.background(MaterialTheme.colorScheme.surface)) {
            SelectableCell(title = "Selected", isSelected = true)
            SelectableCell(title = "Not Selected", isSelected = false)
        }
    }
}

@Composable
fun SelectableCell(
    title: String,
    isSelected: Boolean,
    iconContentDescription: String? = null,
    selectedIcon: @Composable () -> Unit = {
        Icon(
            painter = painterResource(id = R.drawable.icon_tick),
            contentDescription = iconContentDescription,
            tint = MaterialTheme.colorScheme.onSecondary,
            modifier =
                Modifier.padding(end = Dimens.selectableCellTextMargin)
                    .alpha(if (isSelected) AlphaVisible else AlphaInvisible)
        )
    },
    titleStyle: TextStyle = MaterialTheme.typography.labelLarge,
    startPadding: Dp = Dimens.cellStartPadding,
    selectedColor: Color = MaterialTheme.colorScheme.secondary,
    backgroundColor: Color = MaterialTheme.colorScheme.primaryContainer,
    onSelectedColor: Color = MaterialTheme.colorScheme.onSecondary,
    onBackgroundColor: Color = MaterialTheme.colorScheme.onPrimaryContainer,
    onCellClicked: () -> Unit = {}
) {
    BaseCell(
        onCellClicked = onCellClicked,
        title = {
            BaseCellTitle(
                title = title,
                style = titleStyle,
                textColor =
                    if (isSelected) {
                        onSelectedColor
                    } else {
                        onBackgroundColor
                    }
            )
        },
        background =
            if (isSelected) {
                selectedColor
            } else {
                backgroundColor
            },
        startPadding = startPadding,
        iconView = selectedIcon
    )
}
