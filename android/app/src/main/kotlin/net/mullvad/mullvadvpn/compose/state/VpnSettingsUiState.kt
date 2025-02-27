package net.mullvad.mullvadvpn.compose.state

import net.mullvad.mullvadvpn.model.DefaultDnsOptions
import net.mullvad.mullvadvpn.model.QuantumResistantState
import net.mullvad.mullvadvpn.model.SelectedObfuscation
import net.mullvad.mullvadvpn.viewmodel.CustomDnsItem
import net.mullvad.mullvadvpn.viewmodel.StagedDns

sealed interface VpnSettingsUiState {
    val mtu: String
    val isAutoConnectEnabled: Boolean
    val isLocalNetworkSharingEnabled: Boolean
    val isCustomDnsEnabled: Boolean
    val customDnsItems: List<CustomDnsItem>
    val contentBlockersOptions: DefaultDnsOptions
    val isAllowLanEnabled: Boolean
    val selectedObfuscation: SelectedObfuscation
    val quantumResistant: QuantumResistantState

    data class DefaultUiState(
        override val mtu: String = "",
        override val isAutoConnectEnabled: Boolean = false,
        override val isLocalNetworkSharingEnabled: Boolean = false,
        override val isCustomDnsEnabled: Boolean = false,
        override val isAllowLanEnabled: Boolean = false,
        override val customDnsItems: List<CustomDnsItem> = listOf(),
        override val contentBlockersOptions: DefaultDnsOptions = DefaultDnsOptions(),
        override val selectedObfuscation: SelectedObfuscation = SelectedObfuscation.Off,
        override val quantumResistant: QuantumResistantState = QuantumResistantState.Off
    ) : VpnSettingsUiState

    data class MtuDialogUiState(
        override val mtu: String = "",
        override val isAutoConnectEnabled: Boolean = false,
        override val isLocalNetworkSharingEnabled: Boolean = false,
        override val isCustomDnsEnabled: Boolean = false,
        override val isAllowLanEnabled: Boolean = false,
        override val customDnsItems: List<CustomDnsItem> = listOf(),
        override val contentBlockersOptions: DefaultDnsOptions = DefaultDnsOptions(),
        val mtuEditValue: String,
        override val selectedObfuscation: SelectedObfuscation = SelectedObfuscation.Off,
        override val quantumResistant: QuantumResistantState = QuantumResistantState.Off
    ) : VpnSettingsUiState

    data class DnsDialogUiState(
        override val mtu: String = "",
        override val isAutoConnectEnabled: Boolean = false,
        override val isLocalNetworkSharingEnabled: Boolean = false,
        override val isCustomDnsEnabled: Boolean = false,
        override val isAllowLanEnabled: Boolean = false,
        override val customDnsItems: List<CustomDnsItem> = listOf(),
        override val contentBlockersOptions: DefaultDnsOptions = DefaultDnsOptions(),
        val stagedDns: StagedDns,
        override val selectedObfuscation: SelectedObfuscation = SelectedObfuscation.Off,
        override val quantumResistant: QuantumResistantState = QuantumResistantState.Off
    ) : VpnSettingsUiState

    data class LocalNetworkSharingInfoDialogUiState(
        override val mtu: String = "",
        override val isAutoConnectEnabled: Boolean = false,
        override val isLocalNetworkSharingEnabled: Boolean = false,
        override val isCustomDnsEnabled: Boolean = false,
        override val isAllowLanEnabled: Boolean = false,
        override val customDnsItems: List<CustomDnsItem> = listOf(),
        override val contentBlockersOptions: DefaultDnsOptions = DefaultDnsOptions(),
        override val selectedObfuscation: SelectedObfuscation = SelectedObfuscation.Off,
        override val quantumResistant: QuantumResistantState = QuantumResistantState.Off
    ) : VpnSettingsUiState

    data class ContentBlockersInfoDialogUiState(
        override val mtu: String = "",
        override val isAutoConnectEnabled: Boolean = false,
        override val isLocalNetworkSharingEnabled: Boolean = false,
        override val isCustomDnsEnabled: Boolean = false,
        override val isAllowLanEnabled: Boolean = false,
        override val customDnsItems: List<CustomDnsItem> = listOf(),
        override val contentBlockersOptions: DefaultDnsOptions = DefaultDnsOptions(),
        override val selectedObfuscation: SelectedObfuscation = SelectedObfuscation.Off,
        override val quantumResistant: QuantumResistantState = QuantumResistantState.Off
    ) : VpnSettingsUiState

    data class CustomDnsInfoDialogUiState(
        override val mtu: String = "",
        override val isAutoConnectEnabled: Boolean = false,
        override val isLocalNetworkSharingEnabled: Boolean = false,
        override val isCustomDnsEnabled: Boolean = false,
        override val isAllowLanEnabled: Boolean = false,
        override val customDnsItems: List<CustomDnsItem> = listOf(),
        override val contentBlockersOptions: DefaultDnsOptions = DefaultDnsOptions(),
        override val selectedObfuscation: SelectedObfuscation = SelectedObfuscation.Off,
        override val quantumResistant: QuantumResistantState = QuantumResistantState.Off
    ) : VpnSettingsUiState

    data class MalwareInfoDialogUiState(
        override val mtu: String = "",
        override val isAutoConnectEnabled: Boolean = false,
        override val isLocalNetworkSharingEnabled: Boolean = false,
        override val isCustomDnsEnabled: Boolean = false,
        override val isAllowLanEnabled: Boolean = false,
        override val customDnsItems: List<CustomDnsItem> = listOf(),
        override val contentBlockersOptions: DefaultDnsOptions = DefaultDnsOptions(),
        override val selectedObfuscation: SelectedObfuscation = SelectedObfuscation.Off,
        override val quantumResistant: QuantumResistantState = QuantumResistantState.Off
    ) : VpnSettingsUiState

    data class ObfuscationInfoDialogUiState(
        override val mtu: String = "",
        override val isAutoConnectEnabled: Boolean = false,
        override val isLocalNetworkSharingEnabled: Boolean = false,
        override val isCustomDnsEnabled: Boolean = false,
        override val isAllowLanEnabled: Boolean = false,
        override val customDnsItems: List<CustomDnsItem> = listOf(),
        override val contentBlockersOptions: DefaultDnsOptions = DefaultDnsOptions(),
        override val selectedObfuscation: SelectedObfuscation = SelectedObfuscation.Off,
        override val quantumResistant: QuantumResistantState = QuantumResistantState.Off
    ) : VpnSettingsUiState

    data class QuantumResistanceInfoDialogUiState(
        override val mtu: String = "",
        override val isAutoConnectEnabled: Boolean = false,
        override val isLocalNetworkSharingEnabled: Boolean = false,
        override val isCustomDnsEnabled: Boolean = false,
        override val isAllowLanEnabled: Boolean = false,
        override val customDnsItems: List<CustomDnsItem> = listOf(),
        override val contentBlockersOptions: DefaultDnsOptions = DefaultDnsOptions(),
        override val selectedObfuscation: SelectedObfuscation = SelectedObfuscation.Off,
        override val quantumResistant: QuantumResistantState = QuantumResistantState.Off
    ) : VpnSettingsUiState
}
