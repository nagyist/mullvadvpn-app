package net.mullvad.mullvadvpn.compose.screen

import android.widget.Toast
import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.wrapContentHeight
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.Divider
import androidx.compose.material.ExperimentalMaterialApi
import androidx.compose.material.Text
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.dimensionResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.LifecycleOwner
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.distinctUntilChanged
import me.onebone.toolbar.ScrollStrategy
import me.onebone.toolbar.rememberCollapsingToolbarScaffoldState
import net.mullvad.mullvadvpn.R
import net.mullvad.mullvadvpn.compose.cell.BaseCell
import net.mullvad.mullvadvpn.compose.cell.ContentBlockersDisableModeCellSubtitle
import net.mullvad.mullvadvpn.compose.cell.CustomDnsCellSubtitle
import net.mullvad.mullvadvpn.compose.cell.DnsCell
import net.mullvad.mullvadvpn.compose.cell.ExpandableComposeCell
import net.mullvad.mullvadvpn.compose.cell.HeaderSwitchComposeCell
import net.mullvad.mullvadvpn.compose.cell.InformationComposeCell
import net.mullvad.mullvadvpn.compose.cell.MtuComposeCell
import net.mullvad.mullvadvpn.compose.cell.NormalSwitchComposeCell
import net.mullvad.mullvadvpn.compose.cell.SelectableCell
import net.mullvad.mullvadvpn.compose.component.CollapsableAwareToolbarScaffold
import net.mullvad.mullvadvpn.compose.component.CollapsingTopBar
import net.mullvad.mullvadvpn.compose.component.drawVerticalScrollbar
import net.mullvad.mullvadvpn.compose.dialog.ContentBlockersInfoDialog
import net.mullvad.mullvadvpn.compose.dialog.CustomDnsInfoDialog
import net.mullvad.mullvadvpn.compose.dialog.DnsDialog
import net.mullvad.mullvadvpn.compose.dialog.LocalNetworkSharingInfoDialog
import net.mullvad.mullvadvpn.compose.dialog.MalwareInfoDialog
import net.mullvad.mullvadvpn.compose.dialog.MtuDialog
import net.mullvad.mullvadvpn.compose.dialog.ObfuscationInfoDialog
import net.mullvad.mullvadvpn.compose.dialog.QuantumResistanceInfoDialog
import net.mullvad.mullvadvpn.compose.extensions.itemWithDivider
import net.mullvad.mullvadvpn.compose.state.VpnSettingsUiState
import net.mullvad.mullvadvpn.compose.test.LAZY_LIST_LAST_ITEM_TEST_TAG
import net.mullvad.mullvadvpn.compose.test.LAZY_LIST_QUANTUM_ITEM_OFF_TEST_TAG
import net.mullvad.mullvadvpn.compose.test.LAZY_LIST_QUANTUM_ITEM_ON_TEST_TAG
import net.mullvad.mullvadvpn.compose.test.LAZY_LIST_TEST_TAG
import net.mullvad.mullvadvpn.compose.theme.AppTheme
import net.mullvad.mullvadvpn.compose.theme.Dimens
import net.mullvad.mullvadvpn.model.QuantumResistantState
import net.mullvad.mullvadvpn.model.SelectedObfuscation
import net.mullvad.mullvadvpn.viewmodel.CustomDnsItem

@OptIn(ExperimentalMaterialApi::class)
@Preview
@Composable
private fun PreviewVpnSettings() {
    AppTheme {
        VpnSettingsScreen(
            uiState =
                VpnSettingsUiState.DefaultUiState(
                    isAutoConnectEnabled = true,
                    mtu = "1337",
                    isCustomDnsEnabled = true,
                    customDnsItems = listOf(CustomDnsItem("0.0.0.0", false)),
                ),
            onMtuCellClick = {},
            onMtuInputChange = {},
            onSaveMtuClick = {},
            onRestoreMtuClick = {},
            onCancelMtuDialogClick = {},
            onToggleAutoConnect = {},
            onToggleLocalNetworkSharing = {},
            onToggleDnsClick = {},
            onToggleBlockAds = {},
            onToggleBlockTrackers = {},
            onToggleBlockMalware = {},
            onToggleBlockAdultContent = {},
            onToggleBlockGambling = {},
            onDnsClick = {},
            onDnsInputChange = {},
            onSaveDnsClick = {},
            onRemoveDnsClick = {},
            onCancelDnsDialogClick = {},
            onLocalNetworkSharingInfoClick = {},
            onContentsBlockersInfoClick = {},
            onMalwareInfoClick = {},
            onCustomDnsInfoClick = {},
            onDismissInfoClick = {},
            onBackClick = {},
            toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow(),
            onStopEvent = {},
            onSelectObfuscationSetting = {},
            onObfuscationInfoClick = {},
            onSelectQuantumResistanceSetting = {},
            onQuantumResistanceInfoClicked = {}
        )
    }
}

@OptIn(ExperimentalFoundationApi::class)
@ExperimentalMaterialApi
@Composable
fun VpnSettingsScreen(
    lifecycleOwner: LifecycleOwner = LocalLifecycleOwner.current,
    uiState: VpnSettingsUiState,
    onMtuCellClick: () -> Unit = {},
    onMtuInputChange: (String) -> Unit = {},
    onSaveMtuClick: () -> Unit = {},
    onRestoreMtuClick: () -> Unit = {},
    onCancelMtuDialogClick: () -> Unit = {},
    onToggleAutoConnect: (Boolean) -> Unit = {},
    onToggleLocalNetworkSharing: (Boolean) -> Unit = {},
    onToggleDnsClick: (Boolean) -> Unit = {},
    onToggleBlockAds: (Boolean) -> Unit = {},
    onToggleBlockTrackers: (Boolean) -> Unit = {},
    onToggleBlockMalware: (Boolean) -> Unit = {},
    onToggleBlockAdultContent: (Boolean) -> Unit = {},
    onToggleBlockGambling: (Boolean) -> Unit = {},
    onDnsClick: (index: Int?) -> Unit = {},
    onDnsInputChange: (String) -> Unit = {},
    onSaveDnsClick: () -> Unit = {},
    onRemoveDnsClick: () -> Unit = {},
    onCancelDnsDialogClick: () -> Unit = {},
    onLocalNetworkSharingInfoClick: () -> Unit = {},
    onContentsBlockersInfoClick: () -> Unit = {},
    onMalwareInfoClick: () -> Unit = {},
    onCustomDnsInfoClick: () -> Unit = {},
    onDismissInfoClick: () -> Unit = {},
    onBackClick: () -> Unit = {},
    onStopEvent: () -> Unit = {},
    toastMessagesSharedFlow: SharedFlow<String>,
    onSelectObfuscationSetting: (selectedObfuscation: SelectedObfuscation) -> Unit = {},
    onObfuscationInfoClick: () -> Unit = {},
    onSelectQuantumResistanceSetting: (quantumResistant: QuantumResistantState) -> Unit = {},
    onQuantumResistanceInfoClicked: () -> Unit = {}
) {
    val cellVerticalSpacing = dimensionResource(id = R.dimen.cell_label_vertical_padding)
    val cellHorizontalSpacing = dimensionResource(id = R.dimen.cell_left_padding)

    when (uiState) {
        is VpnSettingsUiState.MtuDialogUiState -> {
            MtuDialog(
                mtuValue = uiState.mtuEditValue,
                onMtuValueChanged = { onMtuInputChange(it) },
                onSave = { onSaveMtuClick() },
                onRestoreDefaultValue = { onRestoreMtuClick() },
                onDismiss = { onCancelMtuDialogClick() }
            )
        }
        is VpnSettingsUiState.DnsDialogUiState -> {
            DnsDialog(
                stagedDns = uiState.stagedDns,
                isAllowLanEnabled = uiState.isAllowLanEnabled,
                onIpAddressChanged = { onDnsInputChange(it) },
                onAttemptToSave = { onSaveDnsClick() },
                onRemove = { onRemoveDnsClick() },
                onDismiss = { onCancelDnsDialogClick() }
            )
        }
        is VpnSettingsUiState.LocalNetworkSharingInfoDialogUiState -> {
            LocalNetworkSharingInfoDialog(onDismissInfoClick)
        }
        is VpnSettingsUiState.ContentBlockersInfoDialogUiState -> {
            ContentBlockersInfoDialog(onDismissInfoClick)
        }
        is VpnSettingsUiState.CustomDnsInfoDialogUiState -> {
            CustomDnsInfoDialog(onDismissInfoClick)
        }
        is VpnSettingsUiState.MalwareInfoDialogUiState -> {
            MalwareInfoDialog(onDismissInfoClick)
        }
        is VpnSettingsUiState.ObfuscationInfoDialogUiState -> {
            ObfuscationInfoDialog(onDismissInfoClick)
        }
        is VpnSettingsUiState.QuantumResistanceInfoDialogUiState -> {
            QuantumResistanceInfoDialog(onDismissInfoClick)
        }
        else -> {
            // NOOP
        }
    }

    val lazyListState = rememberLazyListState()
    var expandContentBlockersState by rememberSaveable { mutableStateOf(false) }
    val biggerPadding = 54.dp
    val topPadding = 6.dp
    val state = rememberCollapsingToolbarScaffoldState()
    val progress = state.toolbarState.progress

    CollapsableAwareToolbarScaffold(
        backgroundColor = MaterialTheme.colorScheme.background,
        modifier = Modifier.fillMaxSize(),
        state = state,
        scrollStrategy = ScrollStrategy.ExitUntilCollapsed,
        isEnabledWhenCollapsable = true,
        toolbar = {
            val scaffoldModifier =
                Modifier.road(
                    whenCollapsed = Alignment.TopCenter,
                    whenExpanded = Alignment.BottomStart
                )
            CollapsingTopBar(
                backgroundColor = MaterialTheme.colorScheme.background,
                onBackClicked = { onBackClick() },
                title = stringResource(id = R.string.settings_vpn),
                progress = progress,
                modifier = scaffoldModifier,
                backTitle = stringResource(id = R.string.settings)
            )
        },
    ) {
        val context = LocalContext.current
        LaunchedEffect(Unit) {
            toastMessagesSharedFlow.distinctUntilChanged().collect { message ->
                Toast.makeText(context, message, Toast.LENGTH_SHORT).show()
            }
        }
        DisposableEffect(lifecycleOwner) {
            val observer = LifecycleEventObserver { _, event ->
                if (event == Lifecycle.Event.ON_STOP) {
                    onStopEvent()
                }
            }
            lifecycleOwner.lifecycle.addObserver(observer)
            onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
        }
        LazyColumn(
            modifier =
                Modifier.drawVerticalScrollbar(lazyListState)
                    .testTag(LAZY_LIST_TEST_TAG)
                    .fillMaxWidth()
                    .wrapContentHeight()
                    .animateContentSize(),
            state = lazyListState
        ) {
            item {
                Spacer(modifier = Modifier.height(cellVerticalSpacing))
                HeaderSwitchComposeCell(
                    title = stringResource(R.string.auto_connect),
                    subtitle = stringResource(id = R.string.auto_connect_footer),
                    isToggled = uiState.isAutoConnectEnabled,
                    isEnabled = true,
                    onCellClicked = { newValue -> onToggleAutoConnect(newValue) }
                )
            }
            item {
                Spacer(modifier = Modifier.height(cellVerticalSpacing))
                HeaderSwitchComposeCell(
                    title = stringResource(R.string.local_network_sharing),
                    isToggled = uiState.isAllowLanEnabled,
                    isEnabled = true,
                    onCellClicked = { newValue -> onToggleLocalNetworkSharing(newValue) },
                    onInfoClicked = { onLocalNetworkSharingInfoClick() }
                )
            }
            item {
                Spacer(modifier = Modifier.height(cellVerticalSpacing))
                MtuComposeCell(mtuValue = uiState.mtu, onEditMtu = { onMtuCellClick() })
            }

            itemWithDivider {
                ExpandableComposeCell(
                    title = stringResource(R.string.dns_content_blockers_title),
                    isExpanded = !expandContentBlockersState,
                    isEnabled = !uiState.isCustomDnsEnabled,
                    onInfoClicked = { onContentsBlockersInfoClick() },
                    onCellClicked = { expandContentBlockersState = !expandContentBlockersState }
                )
            }

            if (expandContentBlockersState) {
                itemWithDivider {
                    NormalSwitchComposeCell(
                        title = stringResource(R.string.block_ads_title),
                        isToggled = uiState.contentBlockersOptions.blockAds,
                        isEnabled = !uiState.isCustomDnsEnabled,
                        onCellClicked = { onToggleBlockAds(it) },
                        background = MaterialTheme.colorScheme.secondaryContainer,
                        startPadding = Dimens.indentedCellStartPadding
                    )
                }
                itemWithDivider {
                    NormalSwitchComposeCell(
                        title = stringResource(R.string.block_trackers_title),
                        isToggled = uiState.contentBlockersOptions.blockTrackers,
                        isEnabled = !uiState.isCustomDnsEnabled,
                        onCellClicked = { onToggleBlockTrackers(it) },
                        background = MaterialTheme.colorScheme.secondaryContainer,
                        startPadding = Dimens.indentedCellStartPadding
                    )
                }
                itemWithDivider {
                    NormalSwitchComposeCell(
                        title = stringResource(R.string.block_malware_title),
                        isToggled = uiState.contentBlockersOptions.blockMalware,
                        isEnabled = !uiState.isCustomDnsEnabled,
                        onCellClicked = { onToggleBlockMalware(it) },
                        onInfoClicked = { onMalwareInfoClick() },
                        background = MaterialTheme.colorScheme.secondaryContainer,
                        startPadding = Dimens.indentedCellStartPadding
                    )
                }
                itemWithDivider {
                    NormalSwitchComposeCell(
                        title = stringResource(R.string.block_gambling_title),
                        isToggled = uiState.contentBlockersOptions.blockGambling,
                        isEnabled = !uiState.isCustomDnsEnabled,
                        onCellClicked = { onToggleBlockGambling(it) },
                        background = MaterialTheme.colorScheme.secondaryContainer,
                        startPadding = Dimens.indentedCellStartPadding
                    )
                }
                itemWithDivider {
                    NormalSwitchComposeCell(
                        title = stringResource(R.string.block_adult_content_title),
                        isToggled = uiState.contentBlockersOptions.blockAdultContent,
                        isEnabled = !uiState.isCustomDnsEnabled,
                        onCellClicked = { onToggleBlockAdultContent(it) },
                        background = MaterialTheme.colorScheme.secondaryContainer,
                        startPadding = Dimens.indentedCellStartPadding
                    )
                }

                if (uiState.isCustomDnsEnabled) {
                    item {
                        ContentBlockersDisableModeCellSubtitle(
                            Modifier.background(MaterialTheme.colorScheme.secondary)
                                .padding(
                                    start = cellHorizontalSpacing,
                                    top = topPadding,
                                    end = cellHorizontalSpacing,
                                    bottom = cellVerticalSpacing
                                )
                        )
                    }
                }
            }

            itemWithDivider {
                Spacer(modifier = Modifier.height(cellVerticalSpacing))
                InformationComposeCell(
                    title = stringResource(R.string.obfuscation_title),
                    onInfoClicked = { onObfuscationInfoClick() }
                )
            }
            itemWithDivider {
                SelectableCell(
                    title = stringResource(id = R.string.automatic),
                    isSelected = uiState.selectedObfuscation == SelectedObfuscation.Auto,
                    onCellClicked = { onSelectObfuscationSetting(SelectedObfuscation.Auto) }
                )
            }
            itemWithDivider {
                SelectableCell(
                    title = stringResource(id = R.string.obfuscation_on_udp_over_tcp),
                    isSelected = uiState.selectedObfuscation == SelectedObfuscation.Udp2Tcp,
                    onCellClicked = { onSelectObfuscationSetting(SelectedObfuscation.Udp2Tcp) }
                )
            }
            itemWithDivider {
                SelectableCell(
                    title = stringResource(id = R.string.off),
                    isSelected = uiState.selectedObfuscation == SelectedObfuscation.Off,
                    onCellClicked = { onSelectObfuscationSetting(SelectedObfuscation.Off) }
                )
            }

            itemWithDivider {
                Spacer(modifier = Modifier.height(cellVerticalSpacing))
                InformationComposeCell(
                    title = stringResource(R.string.quantum_resistant_title),
                    onInfoClicked = { onQuantumResistanceInfoClicked() }
                )
            }
            itemWithDivider {
                SelectableCell(
                    title = stringResource(id = R.string.automatic),
                    isSelected = uiState.quantumResistant == QuantumResistantState.Auto,
                    onCellClicked = { onSelectQuantumResistanceSetting(QuantumResistantState.Auto) }
                )
            }
            itemWithDivider {
                SelectableCell(
                    title = stringResource(id = R.string.on),
                    testTag = LAZY_LIST_QUANTUM_ITEM_ON_TEST_TAG,
                    isSelected = uiState.quantumResistant == QuantumResistantState.On,
                    onCellClicked = { onSelectQuantumResistanceSetting(QuantumResistantState.On) }
                )
            }
            itemWithDivider {
                SelectableCell(
                    title = stringResource(id = R.string.off),
                    testTag = LAZY_LIST_QUANTUM_ITEM_OFF_TEST_TAG,
                    isSelected = uiState.quantumResistant == QuantumResistantState.Off,
                    onCellClicked = { onSelectQuantumResistanceSetting(QuantumResistantState.Off) }
                )
            }

            item {
                Spacer(modifier = Modifier.height(cellVerticalSpacing))
                HeaderSwitchComposeCell(
                    title = stringResource(R.string.enable_custom_dns),
                    isToggled = uiState.isCustomDnsEnabled,
                    isEnabled = uiState.contentBlockersOptions.isAnyBlockerEnabled().not(),
                    onCellClicked = { newValue -> onToggleDnsClick(newValue) },
                    onInfoClicked = { onCustomDnsInfoClick() }
                )
            }

            if (uiState.isCustomDnsEnabled) {
                itemsIndexed(uiState.customDnsItems) { index, item ->
                    DnsCell(
                        address = item.address,
                        isUnreachableLocalDnsWarningVisible =
                            item.isLocal && uiState.isAllowLanEnabled.not(),
                        onClick = { onDnsClick(index) },
                        modifier = Modifier.animateItemPlacement()
                    )
                    Divider()
                }

                itemWithDivider {
                    BaseCell(
                        onCellClicked = { onDnsClick(null) },
                        title = {
                            Text(
                                text = stringResource(id = R.string.add_a_server),
                                color = Color.White,
                            )
                        },
                        bodyView = {},
                        subtitle = null,
                        background = MaterialTheme.colorScheme.secondaryContainer,
                        startPadding = biggerPadding,
                    )
                }
            }

            item {
                CustomDnsCellSubtitle(
                    isCellClickable = uiState.contentBlockersOptions.isAnyBlockerEnabled().not(),
                    modifier =
                        Modifier.background(MaterialTheme.colorScheme.secondary)
                            .testTag(LAZY_LIST_LAST_ITEM_TEST_TAG)
                            .padding(
                                start = cellHorizontalSpacing,
                                top = topPadding,
                                end = cellHorizontalSpacing,
                                bottom = cellVerticalSpacing,
                            )
                )
            }
        }
    }
}
