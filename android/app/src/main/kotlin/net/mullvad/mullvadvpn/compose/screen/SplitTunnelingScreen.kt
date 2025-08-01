package net.mullvad.mullvadvpn.compose.screen

import android.graphics.drawable.Drawable
import android.os.Parcelable
import androidx.compose.animation.AnimatedVisibilityScope
import androidx.compose.animation.ExperimentalSharedTransitionApi
import androidx.compose.animation.SharedTransitionScope
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.focus.FocusDirection
import androidx.compose.ui.focus.FocusManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.compose.dropUnlessResumed
import com.ramcosta.composedestinations.annotation.Destination
import com.ramcosta.composedestinations.annotation.RootGraph
import com.ramcosta.composedestinations.navigation.DestinationsNavigator
import kotlinx.parcelize.Parcelize
import net.mullvad.mullvadvpn.R
import net.mullvad.mullvadvpn.applist.AppData
import net.mullvad.mullvadvpn.compose.cell.HeaderCell
import net.mullvad.mullvadvpn.compose.cell.HeaderSwitchComposeCell
import net.mullvad.mullvadvpn.compose.cell.SplitTunnelingCell
import net.mullvad.mullvadvpn.compose.cell.SwitchComposeSubtitleCell
import net.mullvad.mullvadvpn.compose.component.MullvadCircularProgressIndicatorLarge
import net.mullvad.mullvadvpn.compose.component.NavigateBackIconButton
import net.mullvad.mullvadvpn.compose.component.NavigateCloseIconButton
import net.mullvad.mullvadvpn.compose.component.ScaffoldWithMediumTopBar
import net.mullvad.mullvadvpn.compose.component.textResource
import net.mullvad.mullvadvpn.compose.constant.CommonContentKey
import net.mullvad.mullvadvpn.compose.constant.ContentType
import net.mullvad.mullvadvpn.compose.constant.SplitTunnelingContentKey
import net.mullvad.mullvadvpn.compose.extensions.itemWithDivider
import net.mullvad.mullvadvpn.compose.extensions.itemsIndexedWithDivider
import net.mullvad.mullvadvpn.compose.preview.SplitTunnelingUiStatePreviewParameterProvider
import net.mullvad.mullvadvpn.compose.transitions.SlideInFromRightTransition
import net.mullvad.mullvadvpn.lib.model.FeatureIndicator
import net.mullvad.mullvadvpn.lib.theme.AppTheme
import net.mullvad.mullvadvpn.lib.theme.Dimens
import net.mullvad.mullvadvpn.lib.theme.color.AlphaDisabled
import net.mullvad.mullvadvpn.lib.theme.color.AlphaVisible
import net.mullvad.mullvadvpn.util.Lc
import net.mullvad.mullvadvpn.util.getApplicationIconOrNull
import net.mullvad.mullvadvpn.viewmodel.Loading
import net.mullvad.mullvadvpn.viewmodel.SplitTunnelingUiState
import net.mullvad.mullvadvpn.viewmodel.SplitTunnelingViewModel
import org.koin.androidx.compose.koinViewModel

@Preview("ShowAppList|Loading")
@Composable
private fun PreviewSplitTunnelingScreen(
    @PreviewParameter(SplitTunnelingUiStatePreviewParameterProvider::class)
    state: Lc<Loading, SplitTunnelingUiState>
) {
    AppTheme {
        SplitTunnelingScreen(
            state = state,
            onEnableSplitTunneling = {},
            onShowSystemAppsClick = {},
            onExcludeAppClick = {},
            onIncludeAppClick = {},
            onBackClick = {},
            onResolveIcon = { null },
        )
    }
}

@Parcelize data class SplitTunnelingNavArgs(val isModal: Boolean = false) : Parcelable

@OptIn(ExperimentalSharedTransitionApi::class)
@Destination<RootGraph>(
    style = SlideInFromRightTransition::class,
    navArgs = SplitTunnelingNavArgs::class,
)
@Composable
fun SharedTransitionScope.SplitTunneling(
    navigator: DestinationsNavigator,
    animatedVisibilityScope: AnimatedVisibilityScope,
) {
    val viewModel = koinViewModel<SplitTunnelingViewModel>()
    val state by viewModel.uiState.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val packageManager = remember(context) { context.packageManager }

    SplitTunnelingScreen(
        state = state,
        modifier =
            Modifier.sharedBounds(
                rememberSharedContentState(key = FeatureIndicator.SPLIT_TUNNELING),
                animatedVisibilityScope = animatedVisibilityScope,
            ),
        onEnableSplitTunneling = viewModel::onEnableSplitTunneling,
        onShowSystemAppsClick = viewModel::onShowSystemAppsClick,
        onExcludeAppClick = viewModel::onExcludeAppClick,
        onIncludeAppClick = viewModel::onIncludeAppClick,
        onBackClick = dropUnlessResumed { navigator.navigateUp() },
        onResolveIcon = { packageName -> packageManager.getApplicationIconOrNull(packageName) },
    )
}

@Composable
fun SplitTunnelingScreen(
    state: Lc<Loading, SplitTunnelingUiState>,
    onEnableSplitTunneling: (Boolean) -> Unit,
    onShowSystemAppsClick: (show: Boolean) -> Unit,
    onExcludeAppClick: (packageName: String) -> Unit,
    onIncludeAppClick: (packageName: String) -> Unit,
    onBackClick: () -> Unit,
    onResolveIcon: (String) -> Drawable?,
    modifier: Modifier = Modifier,
) {
    val focusManager = LocalFocusManager.current

    ScaffoldWithMediumTopBar(
        modifier = modifier.fillMaxSize(),
        appBarTitle = stringResource(id = R.string.split_tunneling),
        navigationIcon = {
            if (state.isModal()) {
                NavigateCloseIconButton(onNavigateClose = onBackClick)
            } else {
                NavigateBackIconButton(onNavigateBack = onBackClick)
            }
        },
    ) { modifier, lazyListState ->
        LazyColumn(
            modifier = modifier.background(MaterialTheme.colorScheme.surface),
            horizontalAlignment = Alignment.CenterHorizontally,
            state = lazyListState,
        ) {
            description()
            enabledToggle(
                enabled = state.enabled(),
                onEnableSplitTunneling = onEnableSplitTunneling,
            )
            spacer()
            when (state) {
                is Lc.Loading -> {
                    loading()
                }
                is Lc.Content -> {
                    appList(
                        state = state.value,
                        focusManager = focusManager,
                        onShowSystemAppsClick = onShowSystemAppsClick,
                        onExcludeAppClick = onExcludeAppClick,
                        onIncludeAppClick = onIncludeAppClick,
                        onResolveIcon = onResolveIcon,
                    )
                }
            }
        }
    }
}

private fun LazyListScope.enabledToggle(
    enabled: Boolean,
    onEnableSplitTunneling: (Boolean) -> Unit,
) {
    item {
        HeaderSwitchComposeCell(
            title = textResource(id = R.string.enable),
            isToggled = enabled,
            onCellClicked = onEnableSplitTunneling,
        )
    }
}

private fun LazyListScope.description() {
    item(key = CommonContentKey.DESCRIPTION, contentType = ContentType.DESCRIPTION) {
        SwitchComposeSubtitleCell(
            text =
                buildString {
                    appendLine(stringResource(id = R.string.split_tunneling_description))
                    append(stringResource(id = R.string.split_tunneling_description_warning))
                },
            style = MaterialTheme.typography.labelLarge,
        )
    }
}

private fun LazyListScope.loading() {
    item(key = CommonContentKey.PROGRESS, contentType = ContentType.PROGRESS) {
        MullvadCircularProgressIndicatorLarge()
    }
}

private fun LazyListScope.appList(
    state: SplitTunnelingUiState,
    focusManager: FocusManager,
    onShowSystemAppsClick: (show: Boolean) -> Unit,
    onExcludeAppClick: (packageName: String) -> Unit,
    onIncludeAppClick: (packageName: String) -> Unit,
    onResolveIcon: (String) -> Drawable?,
) {
    if (state.excludedApps.isNotEmpty()) {
        headerItem(
            key = SplitTunnelingContentKey.EXCLUDED_APPLICATIONS,
            textId = R.string.exclude_applications,
            enabled = state.enabled,
        )
        appItems(
            apps = state.excludedApps,
            focusManager = focusManager,
            onAppClick = onIncludeAppClick,
            onResolveIcon = onResolveIcon,
            enabled = state.enabled,
            excluded = true,
        )
        spacer()
    }
    systemAppsToggle(
        showSystemApps = state.showSystemApps,
        onShowSystemAppsClick = onShowSystemAppsClick,
        enabled = state.enabled,
    )
    headerItem(
        key = SplitTunnelingContentKey.INCLUDED_APPLICATIONS,
        textId = R.string.all_applications,
        enabled = state.enabled,
    )
    appItems(
        apps = state.includedApps,
        focusManager = focusManager,
        onAppClick = onExcludeAppClick,
        onResolveIcon = onResolveIcon,
        enabled = state.enabled,
        excluded = false,
    )
}

private fun LazyListScope.appItems(
    apps: List<AppData>,
    focusManager: FocusManager,
    onAppClick: (String) -> Unit,
    onResolveIcon: (String) -> Drawable?,
    enabled: Boolean,
    excluded: Boolean,
) {
    itemsIndexedWithDivider(
        items = apps,
        key = { _, listItem -> listItem.packageName },
        contentType = { _, _ -> ContentType.ITEM },
    ) { index, listItem ->
        SplitTunnelingCell(
            title = listItem.name,
            packageName = listItem.packageName,
            isSelected = excluded,
            enabled = enabled,
            modifier =
                Modifier.animateItem()
                    .fillMaxWidth()
                    .alpha(
                        if (enabled) {
                            AlphaVisible
                        } else {
                            AlphaDisabled
                        }
                    ),
            onResolveIcon = onResolveIcon,
        ) {
            // Move focus down unless the clicked item was the last in this
            // section.
            if (index < apps.size - 1) {
                focusManager.moveFocus(FocusDirection.Down)
            } else {
                focusManager.moveFocus(FocusDirection.Up)
            }

            onAppClick(listItem.packageName)
        }
    }
}

private fun LazyListScope.headerItem(key: String, textId: Int, enabled: Boolean) {
    itemWithDivider(key = key, contentType = ContentType.HEADER) {
        HeaderCell(
            modifier =
                Modifier.animateItem()
                    .alpha(
                        if (enabled) {
                            AlphaVisible
                        } else {
                            AlphaDisabled
                        }
                    ),
            text = stringResource(id = textId),
            background = MaterialTheme.colorScheme.primary,
        )
    }
}

private fun LazyListScope.systemAppsToggle(
    showSystemApps: Boolean,
    onShowSystemAppsClick: (show: Boolean) -> Unit,
    enabled: Boolean,
) {
    itemWithDivider(
        key = SplitTunnelingContentKey.SHOW_SYSTEM_APPLICATIONS,
        contentType = ContentType.OTHER_ITEM,
    ) {
        HeaderSwitchComposeCell(
            title = stringResource(id = R.string.show_system_apps),
            isToggled = showSystemApps,
            onCellClicked = { newValue -> onShowSystemAppsClick(newValue) },
            isEnabled = enabled,
            modifier =
                Modifier.animateItem()
                    .alpha(
                        if (enabled) {
                            AlphaVisible
                        } else {
                            AlphaDisabled
                        }
                    ),
        )
    }
}

private fun LazyListScope.spacer() {
    item(contentType = ContentType.SPACER) {
        Spacer(modifier = Modifier.animateItem().height(Dimens.mediumPadding))
    }
}

private fun Lc<Loading, SplitTunnelingUiState>.isModal(): Boolean =
    when (this) {
        is Lc.Loading -> this.value.isModal
        is Lc.Content -> this.value.isModal
    }

private fun Lc<Loading, SplitTunnelingUiState>.enabled(): Boolean =
    when (this) {
        is Lc.Loading -> this.value.enabled
        is Lc.Content -> this.value.enabled
    }
